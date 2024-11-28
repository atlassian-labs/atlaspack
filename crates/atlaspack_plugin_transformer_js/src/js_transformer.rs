use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use anyhow::{anyhow, Error};

use async_trait::async_trait;
use atlaspack_core::plugin::{PluginContext, PluginOptions, TransformerPlugin};
use atlaspack_core::plugin::{TransformContext, TransformResult};
use atlaspack_core::types::browsers::Browsers;
use atlaspack_core::types::engines::EnvironmentFeature;
use atlaspack_core::types::{
  Asset, BuildMode, Diagnostic, ErrorKind, FileType, LogLevel, OutputFormat, SourceType,
};
use glob_match::glob_match;
use parking_lot::RwLock;
use swc_core::atoms::Atom;

use crate::js_transformer_config::{
  InlineEnvironment, JsTransformerConfig, JsTransformerPackageJson,
};
use crate::package_json::{depends_on_react, supports_automatic_jsx_runtime, PackageJson};
use crate::ts_config::{Jsx, Target, TsConfig};

mod conversion;

/// This is a rust only `TransformerPlugin` implementation for JS assets that goes through the
/// default SWC transformer.
///
/// The transformer is part of the `AssetRequest` and is responsible for:
///
/// * Parsing a JS/TS file
/// * Transforming the file using SWC
/// * Analyzing all its `require`/`import`/`export` statements and returning lists of found
///   `Dependency` as well as exported, imported and re-exported symbols (as `Symbol`, usually
///   mapping to a mangled name that the SWC transformer replaced in the source file + the source
///   module and the source name that has been imported)
pub struct AtlaspackJsTransformerPlugin {
  cache: RwLock<Cache>,
  config: JsTransformerConfig,
  options: Arc<PluginOptions>,
  ts_config: Option<TsConfig>,
}

#[derive(Default)]
struct Cache {
  env_variables: EnvVariablesCache,
}

#[derive(Default)]
struct EnvVariablesCache {
  allowlist: Option<HashMap<Atom, Atom>>,
  disabled: Option<HashMap<Atom, Atom>>,
  enabled: Option<HashMap<Atom, Atom>>,
}

impl AtlaspackJsTransformerPlugin {
  pub fn new(ctx: &PluginContext) -> Result<Self, Error> {
    let config = ctx
      .config
      .load_package_json::<JsTransformerPackageJson>()
      .map_or_else(
        |err| {
          let diagnostic = err.downcast_ref::<Diagnostic>();

          if diagnostic.is_some_and(|d| d.kind != ErrorKind::NotFound) {
            return Err(err);
          }

          Ok(JsTransformerConfig::default())
        },
        |config| Ok(config.contents.config.unwrap_or_default()),
      )?;

    let ts_config = ctx
      .config
      .load_json_config::<TsConfig>("tsconfig.json")
      .map(|config| config.contents)
      .map_err(|err| {
        let diagnostic = err.downcast_ref::<Diagnostic>();

        if diagnostic.is_some_and(|d| d.kind != ErrorKind::NotFound) {
          return Err(err);
        }

        Ok(None::<TsConfig>)
      })
      .ok();

    Ok(Self {
      cache: Default::default(),
      config,
      options: ctx.options.clone(),
      ts_config,
    })
  }

  fn env_variables(&self, asset: &Asset) -> HashMap<Atom, Atom> {
    if self.options.env.is_none()
      || self
        .options
        .env
        .as_ref()
        .is_some_and(|vars| vars.is_empty())
    {
      return HashMap::new();
    }

    let env_vars = self.options.env.clone().unwrap_or_default();
    let inline_environment = self
      .config
      .inline_environment
      .clone()
      .unwrap_or(InlineEnvironment::Enabled(asset.is_source));

    match inline_environment {
      InlineEnvironment::Enabled(enabled) => match enabled {
        false => {
          if let Some(vars) = self.cache.read().env_variables.disabled.as_ref() {
            return vars.clone();
          }

          let mut vars: HashMap<Atom, Atom> = HashMap::new();

          if let Some(node_env) = env_vars.get("NODE_ENV") {
            vars.insert("NODE_ENV".into(), node_env.as_str().into());
          }

          if let Some(build_env) = env_vars.get("ATLASPACK_BUILD_ENV") {
            if build_env == "test" {
              vars.insert("ATLASPACK_BUILD_ENV".into(), "test".into());
            }
          }

          self.cache.write().env_variables.disabled = Some(vars.clone());

          vars
        }
        true => {
          if let Some(vars) = self.cache.read().env_variables.enabled.as_ref() {
            return vars.clone();
          }

          let vars = env_vars
            .iter()
            .map(|(key, value)| (key.as_str().into(), value.as_str().into()))
            .collect::<HashMap<Atom, Atom>>();

          self.cache.write().env_variables.enabled = Some(vars.clone());

          vars
        }
      },
      InlineEnvironment::Environments(environments) => {
        if let Some(vars) = self.cache.read().env_variables.allowlist.as_ref() {
          return vars.clone();
        }

        let mut vars: HashMap<Atom, Atom> = HashMap::new();
        for env_glob in environments {
          for (env_var, value) in env_vars
            .iter()
            .filter(|(key, _value)| glob_match(&env_glob, key))
          {
            vars.insert(env_var.as_str().into(), value.as_str().into());
          }
        }

        self.cache.write().env_variables.allowlist = Some(vars.clone());

        vars
      }
    }
  }
}

impl fmt::Debug for AtlaspackJsTransformerPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("AtlaspackJsTransformerPlugin")
      .field("options", &self.options)
      .finish()
  }
}

#[async_trait]
impl TransformerPlugin for AtlaspackJsTransformerPlugin {
  /// This does equivalent work to `JSTransformer::transform` in `packages/transformers/js`
  async fn transform(
    &self,
    context: TransformContext,
    asset: Asset,
  ) -> Result<TransformResult, Error> {
    let env = asset.env.clone();
    let file_type = asset.file_type.clone();
    let is_node = env.context.is_node();
    let source_code = asset.code.clone();

    let mut targets: HashMap<String, String> = HashMap::new();
    if env.context.is_browser() {
      let browsers = env.engines.browsers.clone().unwrap_or_default();
      let browsers = Browsers::from(&browsers);
      for (name, version) in browsers.iter() {
        targets.insert(
          String::from(name),
          format!("{}.{}", version.major(), version.minor()),
        );
      }
    }

    if env.context.is_electron() {
      if let Some(version) = env.engines.electron {
        targets.insert(
          String::from("electron"),
          format!("{}.{}", version.major(), version.minor()),
        );
      }
    }

    if env.context.is_node() {
      if let Some(version) = env.engines.node {
        targets.insert(
          String::from("node"),
          format!("{}.{}", version.major(), version.minor()),
        );
      }
    }

    let env_vars = self.env_variables(&asset);

    let compiler_options = self
      .ts_config
      .as_ref()
      .and_then(|ts| ts.compiler_options.as_ref());

    let package_json = context.config().load_package_json::<PackageJson>().ok();

    let automatic_jsx_runtime = compiler_options
      .map(|co| {
        co.jsx
          .as_ref()
          .is_some_and(|jsx| matches!(jsx, Jsx::ReactJsx | Jsx::ReactJsxDev))
          || co.jsx_import_source.is_some()
      })
      .unwrap_or_else(|| {
        package_json
          .as_ref()
          .is_some_and(|pkg| supports_automatic_jsx_runtime(&pkg.contents))
      });

    let transform_config = atlaspack_js_swc_core::Config {
      automatic_jsx_runtime,
      code: source_code.bytes().to_vec(),
      decorators: compiler_options
        .and_then(|co| co.experimental_decorators)
        .unwrap_or_default(),
      env: env_vars,
      filename: asset
        .file_path
        .to_str()
        .ok_or_else(|| anyhow!("Invalid non UTF-8 file-path"))?
        .to_string(),
      inline_constants: self.config.inline_constants.unwrap_or_default(),
      inline_fs: !env.context.is_node() && self.config.inline_fs.unwrap_or(true),
      insert_node_globals: !is_node && env.source_type != SourceType::Script,
      is_browser: env.context.is_browser(),
      is_development: self.options.mode == BuildMode::Development,
      is_esm_output: env.output_format == OutputFormat::EsModule,
      is_jsx: matches!(file_type, FileType::Jsx | FileType::Tsx),
      is_library: env.is_library,
      is_type_script: matches!(file_type, FileType::Ts | FileType::Tsx),
      is_worker: env.context.is_worker(),
      jsx_import_source: compiler_options
        .and_then(|co| co.jsx_import_source.clone())
        .or_else(|| automatic_jsx_runtime.then_some(String::from("react"))),
      jsx_pragma: compiler_options.and_then(|co| co.jsx_factory.clone()),
      jsx_pragma_frag: compiler_options.and_then(|co| co.jsx_fragment_factory.clone()),
      magic_comments: self.config.magic_comments.unwrap_or_default(),
      module_id: asset.id.to_string(),
      node_replacer: is_node,
      project_root: self.options.project_root.to_string_lossy().into_owned(),
      react_refresh: self.options.mode == BuildMode::Development && package_json.is_some_and(|pkg| depends_on_react(&pkg.contents))
      // && TODO: self.options.hmr_options
      && env.context.is_browser()
      && !env.is_library
      && !env.context.is_worker()
      && !env.context.is_worklet(),
      replace_env: !is_node,
      scope_hoist: env.should_scope_hoist && env.source_type != SourceType::Script,
      source_maps: env.source_map.is_some(),
      source_type: match env.source_type {
        SourceType::Module => atlaspack_js_swc_core::SourceType::Module,
        SourceType::Script => atlaspack_js_swc_core::SourceType::Script,
      },
      supports_module_workers: env.should_scope_hoist
        && env.engines.supports(EnvironmentFeature::WorkerModule),
      targets: (!targets.is_empty()).then_some(targets),
      trace_bailouts: self.options.log_level == LogLevel::Verbose,
      use_define_for_class_fields: compiler_options
        .map(|co| {
          co.use_define_for_class_fields.unwrap_or_else(|| {
            // Default useDefineForClassFields to true if target is ES2022 or higher (including ESNext)
            co.target.as_ref().is_some_and(|target| {
              matches!(target, Target::ES2022 | Target::ES2023 | Target::ESNext)
            })
          })
        })
        .unwrap_or_default(),
      ..atlaspack_js_swc_core::Config::default()
    };

    let transformation_result = atlaspack_js_swc_core::transform(transform_config, None)?;

    // TODO handle errors properly
    if let Some(errors) = transformation_result.diagnostics {
      return Err(anyhow!(format!("{:#?}", errors)));
    }

    let config = atlaspack_js_swc_core::Config::default();
    let result = conversion::convert_result(asset, &config, transformation_result, &self.options)
      // TODO handle errors properly
      .map_err(|_err| anyhow!("Failed to transform"))?;

    Ok(result)
  }
}

#[cfg(test)]
mod tests {
  use std::path::{Path, PathBuf};

  use atlaspack_core::{
    config_loader::ConfigLoader,
    plugin::PluginLogger,
    types::{
      Code, Dependency, Environment, EnvironmentContext, Location, SourceLocation, SpecifierType,
      Symbol,
    },
  };
  use atlaspack_filesystem::{in_memory_file_system::InMemoryFileSystem, FileSystemRef};
  use pretty_assertions::assert_eq;
  use swc_core::ecma::parser::lexer::util::CharExt;

  use super::*;

  #[tokio::test(flavor = "multi_thread")]
  async fn test_transformer_on_noop_asset() -> anyhow::Result<()> {
    let project_root = Path::new("/root");
    let target_asset =
      create_asset_at_project_root(project_root, "mock_path.js", "function hello() {}");

    assert_eq!(
      run_test(TestOptions {
        asset: target_asset.clone(),
        project_root: Some(project_root.to_path_buf()),
        ..TestOptions::default()
      })
      .await?,
      TransformResult {
        asset: Asset {
          code: Code::from(String::from("function hello() {}")),
          file_path: "mock_path.js".into(),
          file_type: FileType::Js,
          symbols: Some(Vec::new()),
          meta: serde_json::Map::from_iter([(String::from("id"), target_asset.id.clone().into())]),
          ..target_asset
        },
        discovered_assets: vec![],
        dependencies: vec![],
        invalidate_on_file_change: vec![]
      }
    );

    Ok(())
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn transforms_react_jsx() -> anyhow::Result<()> {
    let target_asset = create_asset(
      "index.jsx",
      r"
        import React from 'react';

        const main = () => <div />;
      ",
    );

    let result = run_test(TestOptions {
      asset: target_asset.clone(),
      ..TestOptions::default()
    })
    .await?;

    assert_eq!(
      get_dependencies(&result),
      vec!["react", "@atlaspack/transformer-js/src/esmodule-helpers.js"]
    );

    assert_eq!(
      result.asset,
      Asset {
        code: Code::from(normalize_code(
          r#"
            var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
            var _react = require("react");
            var _reactDefault = parcelHelpers.interopDefault(_react);
            const main = ()=>/*#__PURE__*/ (0, _reactDefault.default).createElement("div", {
              __source: {
                fileName: "index.jsx",
                lineNumber: 4,
                columnNumber: 28
              },
              __self: undefined
            });
          "#
        )),
        file_path: PathBuf::from("index.jsx"),
        file_type: FileType::Js,
        symbols: Some(Vec::new()),
        meta: serde_json::Map::from_iter([(String::from("id"), target_asset.id.clone().into())]),
        ..target_asset
      },
    );

    Ok(())
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn transforms_react_tsx() -> anyhow::Result<()> {
    let target_asset = create_asset(
      "index.tsx",
      r"
        import React, { type FC } from 'react';

        const main: FC = () => <div />;
      ",
    );

    let result = run_test(TestOptions {
      asset: target_asset.clone(),
      ..TestOptions::default()
    })
    .await?;

    assert_eq!(
      get_dependencies(&result),
      vec!["react", "@atlaspack/transformer-js/src/esmodule-helpers.js"]
    );

    assert_eq!(
      result.asset,
      Asset {
        code: Code::from(normalize_code(
          r#"
            var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
            var _react = require("react");
            var _reactDefault = parcelHelpers.interopDefault(_react);
            const main = ()=>/*#__PURE__*/ (0, _reactDefault.default).createElement("div", {
              __source: {
                fileName: "index.tsx",
                lineNumber: 4,
                columnNumber: 32
              },
              __self: undefined
            });
          "#
        )),
        file_path: PathBuf::from("index.tsx"),
        file_type: FileType::Js,
        symbols: Some(Vec::new()),
        meta: serde_json::Map::from_iter([(String::from("id"), target_asset.id.clone().into())]),
        ..target_asset
      },
    );

    Ok(())
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn transforms_react_with_inferred_automatic_runtime_from_package_json() -> anyhow::Result<()>
  {
    async fn test_version(version: &str) -> anyhow::Result<()> {
      let file_system = Arc::new(InMemoryFileSystem::default());
      let target_asset = create_asset("index.jsx", "const main = () => <div />;");

      file_system.write_file(
        Path::new("package.json"),
        format!(r#"{{ "dependencies": {{ "react": "{version}" }} }}"#,),
      );

      let result = run_test(TestOptions {
        asset: target_asset.clone(),
        file_system: Some(file_system),
        ..TestOptions::default()
      })
      .await?;

      assert_eq!(get_dependencies(&result), vec!["react/jsx-dev-runtime"]);

      assert_eq!(
        result.asset,
        Asset {
          code: Code::from(normalize_code(
            r#"
              var _jsxDevRuntime = require("react/jsx-dev-runtime");
              const main = ()=>/*#__PURE__*/ (0, _jsxDevRuntime.jsxDEV)("div", {}, void 0, false, {
                fileName: "index.jsx",
                lineNumber: 1,
                columnNumber: 20
              }, undefined);
            "#
          )),
          file_path: PathBuf::from("index.jsx"),
          file_type: FileType::Js,
          // TODO: Is this correct?
          symbols: Some(vec![Symbol {
            local: String::from("$794b991511cb8fe6$exports"),
            exported: String::from("*"),
            loc: None,
            is_weak: false,
            is_esm_export: false,
            self_referenced: false,
          },]),
          meta: serde_json::Map::from_iter([(String::from("id"), target_asset.id.clone().into())]),
          ..target_asset
        },
      );

      Ok(())
    }

    test_version("^16.14.0").await?;
    test_version("^18.0.0").await?;

    Ok(())
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn transforms_react_with_jsx_pragma() -> anyhow::Result<()> {
    let file_system = Arc::new(InMemoryFileSystem::default());
    let target_asset = create_asset(
      "src/index.jsx",
      "
        /* @jsx jsx */
        import { jsx } from '@emotion/react';

        const main = () => <div />;
      ",
    );

    // This test will fail if the react version is read from the project root
    file_system.write_file(
      Path::new("src/package.json"),
      format!(r#"{{ "dependencies": {{ "react": "^16.0.0" }} }}"#),
    );

    file_system.write_file(
      Path::new("package.json"),
      format!(r#"{{ "dependencies": {{ "react": "^18.0.0" }} }}"#),
    );

    let result = run_test(TestOptions {
      asset: target_asset.clone(),
      file_system: Some(file_system),
      ..TestOptions::default()
    })
    .await?;

    assert_eq!(get_dependencies(&result), vec!["@emotion/react"]);

    assert_eq!(
      result.asset,
      Asset {
        code: Code::from(normalize_code(
          r#"
            /* @jsx jsx */ var _react = require("@emotion/react");
            const main = ()=>/*#__PURE__*/ (0, _react.jsx)("div", {
              __source: {
                fileName: "src/index.jsx",
                lineNumber: 5,
                columnNumber: 28
              },
              __self: undefined
            });
          "#
        )),
        file_path: PathBuf::from("src").join("index.jsx"),
        file_type: FileType::Js,
        symbols: Some(Vec::new()),
        meta: serde_json::Map::from_iter([(String::from("id"), target_asset.id.clone().into())]),
        ..target_asset
      },
    );

    Ok(())
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn transforms_react_with_jsx_runtime_pragma() -> anyhow::Result<()> {
    let file_system = Arc::new(InMemoryFileSystem::default());
    let target_asset = create_asset(
      "index.jsx",
      "
        /**
         * @jsxRuntime classic
         * @jsx jsx
         */

        import { jsx } from '@emotion/react';

        const main = () => <div />;
      ",
    );

    file_system.write_file(
      Path::new("package.json"),
      format!(r#"{{ "dependencies": {{ "react": "^18.0.0" }} }}"#),
    );

    let result = run_test(TestOptions {
      asset: target_asset.clone(),
      file_system: Some(file_system),
      ..TestOptions::default()
    })
    .await?;

    assert_eq!(get_dependencies(&result), vec!["@emotion/react"]);

    assert_eq!(
      result.asset,
      Asset {
        code: Code::from(normalize_code(
          r#"
            /**
               * @jsxRuntime classic
               * @jsx jsx
               */ var _react = require("@emotion/react");
            const main = ()=>/*#__PURE__*/ (0, _react.jsx)("div", {
              __source: {
                fileName: "index.jsx",
                lineNumber: 9,
                columnNumber: 28
              },
              __self: undefined
            });
          "#
        )),
        file_path: PathBuf::from("index.jsx"),
        file_type: FileType::Js,
        symbols: Some(Vec::new()),
        meta: serde_json::Map::from_iter([(String::from("id"), target_asset.id.clone().into())]),
        ..target_asset
      },
    );

    Ok(())
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn uses_latest_node_when_no_node_target() -> anyhow::Result<()> {
    let target_asset = Asset {
      env: Arc::new(Environment {
        context: EnvironmentContext::Node,
        ..Environment::default()
      }),
      ..create_asset("index.js", "const test = () => {};")
    };

    assert_eq!(
      run_test(TestOptions {
        asset: target_asset.clone(),
        ..TestOptions::default()
      })
      .await?,
      TransformResult {
        asset: Asset {
          // This asserts that the code has not been downlevelled into `var test = function() {}`
          code: Code::from(String::from("const test = ()=>{};")),
          file_path: "index.js".into(),
          symbols: Some(Vec::new()),
          meta: serde_json::Map::from_iter([(String::from("id"), target_asset.id.clone().into())]),
          ..target_asset
        },
        discovered_assets: vec![],
        dependencies: vec![],
        invalidate_on_file_change: vec![]
      }
    );

    Ok(())
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_transformer_on_asset_that_requires_other() -> anyhow::Result<()> {
    let source_code = r#"
      const x = require('other');
      exports.hello = function() {};
    "#;

    let project_root = Path::new("/root");
    let target_asset = create_asset_at_project_root(project_root, "mock_path.js", source_code);
    let asset_id = target_asset.id.clone();

    let mut expected_dependencies = vec![Dependency {
      loc: Some(SourceLocation {
        file_path: PathBuf::from("mock_path.js"),
        start: Location {
          line: 2,
          column: 25,
        },
        end: Location {
          line: 2,
          column: 32,
        },
      }),
      placeholder: Some("e83f3db3d6f57ea6".to_string()),
      source_asset_id: Some(asset_id.clone()),
      source_path: Some(PathBuf::from("mock_path.js")),
      source_asset_type: Some(FileType::Js),
      specifier: String::from("other"),
      specifier_type: SpecifierType::CommonJS,
      symbols: Some(vec![Symbol {
        exported: String::from("*"),
        loc: None,
        local: String::from("a1ad9714284f3ad6$"),
        ..Symbol::default()
      }]),
      ..Default::default()
    }];

    expected_dependencies[0].set_placeholder("e83f3db3d6f57ea6");
    expected_dependencies[0].set_kind("Require");

    assert_eq!(
      run_test(TestOptions {
        asset: target_asset,
        project_root: Some(project_root.to_path_buf()),
        ..TestOptions::default()
      })
      .await?,
      TransformResult {
        asset: Asset {
          id: asset_id.clone(),
          file_path: "mock_path.js".into(),
          file_type: FileType::Js,
          // SWC inserts a newline here
          code: Code::from(String::from(
            "const x = require(\"e83f3db3d6f57ea6\");\nexports.hello = function() {};"
          )),
          symbols: Some(vec![
            Symbol {
              exported: String::from("hello"),
              loc: Some(SourceLocation {
                file_path: PathBuf::from("mock_path.js"),
                start: Location {
                  line: 3,
                  column: 15
                },
                end: Location {
                  line: 3,
                  column: 20
                }
              }),
              local: String::from("$hello"),
              ..Default::default()
            },
            Symbol {
              exported: String::from("*"),
              loc: Some(SourceLocation {
                file_path: PathBuf::from("mock_path.js"),
                start: Location { line: 1, column: 1 },
                end: Location { line: 1, column: 1 }
              }),
              local: String::from("$_"),
              ..Default::default()
            },
            Symbol {
              exported: String::from("*"),
              loc: None,
              local: format!("${asset_id}$exports"),
              ..Default::default()
            }
          ]),
          unique_key: None,
          meta: serde_json::Map::from_iter([(String::from("id"), asset_id.clone().into())]),
          ..empty_asset()
        },
        discovered_assets: vec![],
        dependencies: expected_dependencies,
        invalidate_on_file_change: vec![]
      }
    );

    Ok(())
  }

  fn default_fs(project_root: &Path) -> FileSystemRef {
    let file_system = Arc::new(InMemoryFileSystem::default());

    file_system.write_file(&project_root.join("package.json"), String::from("{}"));

    file_system
  }

  fn create_asset(file_path: &str, code: &str) -> Asset {
    let env = Arc::new(Environment::default());

    Asset::new(
      Code::from(code),
      env.clone(),
      file_path.into(),
      None,
      &PathBuf::default(),
      None,
      false,
    )
  }

  fn create_asset_at_project_root(project_root: &Path, file_path: &str, code: &str) -> Asset {
    let env = Arc::new(Environment::default());

    Asset::new(
      Code::from(code),
      env.clone(),
      file_path.into(),
      None,
      project_root,
      None,
      false,
    )
  }

  fn empty_asset() -> Asset {
    Asset {
      file_type: FileType::Js,
      is_bundle_splittable: true,
      is_source: true,
      ..Default::default()
    }
  }

  #[derive(Default)]
  struct TestOptions {
    asset: Asset,
    file_system: Option<FileSystemRef>,
    project_root: Option<PathBuf>,
  }

  async fn run_test(options: TestOptions) -> anyhow::Result<TransformResult> {
    let asset = options.asset;
    let project_root = options.project_root.unwrap_or_default();
    let file_system = options
      .file_system
      .unwrap_or_else(|| default_fs(&project_root));

    let ctx = PluginContext {
      config: Arc::new(ConfigLoader {
        fs: file_system.clone(),
        project_root: project_root.to_path_buf(),
        search_path: project_root.to_path_buf(),
      }),
      file_system: file_system.clone(),
      logger: PluginLogger::default(),
      options: Arc::new(PluginOptions::default()),
    };

    let transformer = AtlaspackJsTransformerPlugin::new(&ctx)?;
    let context = TransformContext::new(
      Arc::new(ConfigLoader {
        fs: file_system,
        project_root,
        search_path: asset.file_path.clone(),
      }),
      Arc::new(Environment::default()),
    );

    let mut result = transformer.transform(context, asset).await?;

    result.asset.code = Code::from(normalize_code(&result.asset.code.as_str()?));

    Ok(result)
  }

  fn get_dependencies(transformation: &TransformResult) -> Vec<String> {
    transformation
      .dependencies
      .iter()
      .map(|d| d.specifier.clone())
      .collect::<Vec<String>>()
  }

  fn normalize_code(code: &str) -> String {
    let code = code
      .trim_start_matches(|c: char| c.is_line_break())
      .trim_end();

    let base_indent = code
      .lines()
      .map(|l| l.find(|c: char| !c.is_whitespace()).unwrap_or_default())
      .filter(|w| w > &0)
      .min()
      .unwrap_or_default();

    if base_indent == 0 {
      assert!(!code.is_empty());
      return code.to_string();
    }

    let code = code
      .lines()
      .map(|l| {
        let whitespace = l.find(|c: char| !c.is_whitespace()).unwrap_or_default();
        let indent = match whitespace / base_indent {
          0 | 1 => String::default(),
          x => "  ".repeat(x - 1),
        };

        let spaces = " ".repeat(whitespace % base_indent);

        format!("{indent}{spaces}{}\n", l.trim())
      })
      .collect::<String>();

    assert!(!code.is_empty());

    code
  }
}
