use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use anyhow::{Error, anyhow};

use async_trait::async_trait;
use atlaspack_core::config_loader::ConfigFile;
use atlaspack_core::plugin::{PluginContext, PluginOptions, TransformerPlugin};
use atlaspack_core::plugin::{TransformContext, TransformResult};
use atlaspack_core::types::browsers::Browsers;
use atlaspack_core::types::engines::EnvironmentFeature;
use atlaspack_core::types::{
  Asset, BuildMode, Diagnostic, Diagnostics, ErrorKind, FileType, LogLevel, OutputFormat,
  SourceType,
};
use atlaspack_js_swc_core::SyncDynamicImportConfig;
use glob_match::glob_match;
use parking_lot::RwLock;
use swc_core::atoms::Atom;

use crate::js_transformer_config::{
  InlineEnvironment, JsTransformerConfig, JsTransformerPackageJson,
};
use crate::map_diagnostics::{MapDiagnosticOptions, map_diagnostics};
use crate::package_json::{PackageJson, depends_on_react, supports_automatic_jsx_runtime};
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
  sync_dynamic_import_config: Option<SyncDynamicImportConfig>,
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

  /// Determines JSX configuration based on file type, tsconfig, and package.json
  fn determine_jsx_configuration(
    &self,
    file_type: &FileType,
    asset: &Asset,
    compiler_options: Option<&crate::ts_config::CompilerOptions>,
    package_json: &Option<ConfigFile<PackageJson>>,
  ) -> (bool, Option<String>, Option<String>, Option<String>, bool) {
    let mut is_jsx = matches!(file_type, FileType::Jsx | FileType::Tsx);
    let mut jsx_pragma = None;
    let mut jsx_pragma_frag = None;
    let mut jsx_import_source = None;
    let mut automatic_jsx_runtime = false;

    if asset.is_source {
      // Check for React dependencies in package.json
      let has_react = package_json
        .as_ref()
        .is_some_and(|pkg| depends_on_react(&pkg.contents));

      // Set up React JSX pragmas if React is present
      if has_react {
        jsx_pragma = Some("React.createElement".to_string());
        jsx_pragma_frag = Some("React.Fragment".to_string());
      }

      // Check tsconfig.json/jsconfig.json for explicit JSX configuration
      if let Some(co) = compiler_options {
        // Override pragmas with explicit configuration
        if let Some(factory) = &co.jsx_factory {
          jsx_pragma = Some(factory.clone());
        }
        if let Some(fragment_factory) = &co.jsx_fragment_factory {
          jsx_pragma_frag = Some(fragment_factory.clone());
        }

        // Check for automatic JSX runtime
        if matches!(co.jsx, Some(Jsx::ReactJsx) | Some(Jsx::ReactJsxDev))
          || co.jsx_import_source.is_some()
        {
          automatic_jsx_runtime = true;
          jsx_import_source = co.jsx_import_source.clone();
        }
      }

      // Determine if JSX should be enabled based on file type and configuration
      match file_type {
        FileType::Jsx | FileType::Tsx => {
          // .jsx and .tsx files should always have JSX enabled
          is_jsx = true;
        }
        FileType::Js => {
          // For .js files, enable JSX if we have configuration (tsconfig or React dependencies)
          is_jsx = compiler_options
            .as_ref()
            .and_then(|co| co.jsx.as_ref())
            .is_some()
            || jsx_pragma.is_some();
        }
        FileType::Ts => {
          // TypeScript files without .tsx extension should not have JSX
          is_jsx = false;
        }
        _ => {}
      }
    }

    // Update automatic_jsx_runtime based on package.json if not set by tsconfig
    if !automatic_jsx_runtime {
      automatic_jsx_runtime = package_json
        .as_ref()
        .is_some_and(|pkg| supports_automatic_jsx_runtime(&pkg.contents));
    }

    (
      is_jsx,
      jsx_pragma,
      jsx_pragma_frag,
      jsx_import_source,
      automatic_jsx_runtime,
    )
  }

  fn env_variables(&self, asset: &Asset) -> HashMap<Atom, Atom> {
    if self.options.env.is_none()
      || self
        .options
        .env
        .as_ref()
        .is_some_and(|vars| vars.is_empty())
    {
      // Still check for custom env even if global env is empty
      if asset.env.custom_env.is_none() {
        return HashMap::new();
      }
    }

    // Merge global environment variables with asset's custom environment variables
    let mut env_vars = self.options.env.clone().unwrap_or_default();
    if let Some(custom_env) = &asset.env.custom_env {
      env_vars.extend(custom_env.clone());
    }
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

          if let Some(build_env) = env_vars.get("ATLASPACK_BUILD_ENV")
            && build_env == "test"
          {
            vars.insert("ATLASPACK_BUILD_ENV".into(), "test".into());
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

  fn sync_dynamic_import_config(&self) -> Option<SyncDynamicImportConfig> {
    if let Some(env_vars) = &self.options.env
      && let Some(config_json) = env_vars.get("SYNC_DYNAMIC_IMPORT_CONFIG")
    {
      if let Some(cached) = self.cache.read().sync_dynamic_import_config.as_ref() {
        return Some(cached.clone());
      }

      match serde_json::from_str::<SyncDynamicImportConfig>(config_json) {
        Ok(config) => {
          self.cache.write().sync_dynamic_import_config = Some(config.clone());
          return Some(config);
        }
        Err(_) => {
          eprintln!(
            "Failed to parse SYNC_DYNAMIC_IMPORT_CONFIG to JSON or config shape did not match. Config will not be applied."
          );

          let fallback = SyncDynamicImportConfig {
            entrypoint_filepath_suffix: "__NO_MATCH__".into(),
            actual_require_paths: vec![],
          };

          self.cache.write().sync_dynamic_import_config = Some(fallback.clone());

          return Some(fallback);
        }
      }
    }

    None
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
  fn cache_key(&self) -> &atlaspack_core::plugin::CacheStatus {
    &atlaspack_core::plugin::CacheStatus::BuiltIn
  }

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

    let feature_flag_conditional_bundling = self
      .options
      .feature_flags
      .bool_enabled("conditionalBundlingApi");

    let feature_flag_hmr_improvements = self.options.feature_flags.bool_enabled("hmrImprovements");

    let feature_flag_exports_rebinding_optimisation = self
      .options
      .feature_flags
      .bool_enabled("exportsRebindingOptimisation");

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

    if env.context.is_electron()
      && let Some(version) = env.engines.electron
    {
      targets.insert(
        String::from("electron"),
        format!("{}.{}", version.major(), version.minor()),
      );
    }

    if env.context.is_node()
      && let Some(version) = env.engines.node
    {
      targets.insert(
        String::from("node"),
        format!("{}.{}", version.major(), version.minor()),
      );
    }

    let env_vars = self.env_variables(&asset);

    let sync_dynamic_import_config = if env.context.is_tesseract() {
      self.sync_dynamic_import_config()
    } else {
      None
    };

    let compiler_options = self
      .ts_config
      .as_ref()
      .and_then(|ts| ts.compiler_options.as_ref());

    let package_json = context.config().load_package_json::<PackageJson>().ok();

    // v3JsxConfigurationLoading CLEANUP NOTE: Remove the flag disabled code path after rollout
    let feature_flag_v3_jsx_configuration_loading = self
      .options
      .feature_flags
      .bool_enabled("v3JsxConfigurationLoading");

    // Determine JSX configuration based on v3JsxConfigurationLoading feature flag
    let (is_jsx, jsx_pragma, jsx_pragma_frag, jsx_import_source, automatic_jsx_runtime) =
      if feature_flag_v3_jsx_configuration_loading {
        // With v3JsxConfigurationLoading enabled, use the new determine_jsx_configuration method
        self.determine_jsx_configuration(&file_type, &asset, compiler_options, &package_json)
      } else {
        // With v3JsxConfigurationLoading disabled, use the old logic
        let is_jsx = matches!(file_type, FileType::Jsx | FileType::Tsx);

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

        let jsx_import_source = compiler_options
          .and_then(|co| co.jsx_import_source.clone())
          .or_else(|| automatic_jsx_runtime.then_some(String::from("react")));

        let jsx_pragma = compiler_options.and_then(|co| co.jsx_factory.clone());
        let jsx_pragma_frag = compiler_options.and_then(|co| co.jsx_fragment_factory.clone());

        (
          is_jsx,
          jsx_pragma,
          jsx_pragma_frag,
          jsx_import_source,
          automatic_jsx_runtime,
        )
      };

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
      is_jsx,
      is_library: env.is_library,
      is_type_script: matches!(file_type, FileType::Ts | FileType::Tsx),
      is_worker: env.context.is_worker(),
      jsx_import_source,
      jsx_pragma,
      jsx_pragma_frag,
      magic_comments: self.config.magic_comments.unwrap_or_default(),
      add_display_name: self.config.add_react_display_name,
      module_id: asset.id.to_string(),
      node_replacer: is_node,
      project_root: self.options.project_root.to_string_lossy().into_owned(),
      react_refresh: self.options.hmr_options.is_some()
        && self.options.mode == BuildMode::Development
        && package_json.is_some_and(|pkg| depends_on_react(&pkg.contents))
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
      conditional_bundling: feature_flag_conditional_bundling,
      hmr_improvements: feature_flag_hmr_improvements,
      exports_rebinding_optimisation: feature_flag_exports_rebinding_optimisation,
      nested_promise_import_fix: self
        .options
        .feature_flags
        .bool_enabled("nestedPromiseImportFix"),
      sync_dynamic_import_config,
      ..atlaspack_js_swc_core::Config::default()
    };

    let transformation_result = atlaspack_js_swc_core::transform(&transform_config, None)?;

    if let Some(errors) = transformation_result.diagnostics {
      return Err(anyhow!(map_diagnostics(
        errors,
        MapDiagnosticOptions {
          source_code: Some(source_code.clone()),
          file_type: Some(file_type.clone()),
          file_path: Some(asset.file_path),
        },
      )));
    }

    let result = conversion::convert_result(
      asset,
      &transform_config,
      transformation_result,
      &self.options,
    )
    .map_err(|errors| anyhow!(Diagnostics::from(errors)))?;

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
      Code, DependencyBuilder, DependencyKind, Environment, EnvironmentContext, FeatureFlags,
      Location, Priority, SourceLocation, SpecifierType, Symbol,
    },
  };
  use atlaspack_filesystem::{FileSystemRef, in_memory_file_system::InMemoryFileSystem};
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
          packaging_id: Some(target_asset.id.to_string()),
          ..target_asset
        },
        discovered_assets: vec![],
        dependencies: vec![],
        invalidate_on_file_change: vec![],
        cache_bailout: false,
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
        packaging_id: Some(target_asset.id.to_string()),
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
        packaging_id: Some(target_asset.id.to_string()),
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
            is_static_binding_safe: false,
          },]),
          packaging_id: Some(target_asset.id.to_string()),
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
      r#"{ "dependencies": { "react": "^16.0.0" } }"#.to_string(),
    );

    file_system.write_file(
      Path::new("package.json"),
      r#"{ "dependencies": { "react": "^18.0.0" } }"#.to_string(),
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
        packaging_id: Some(target_asset.id.to_string()),
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
      r#"{ "dependencies": { "react": "^18.0.0" } }"#.to_string(),
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
        packaging_id: Some(target_asset.id.to_string()),
        ..target_asset
      },
    );

    Ok(())
  }

  #[test]
  fn test_determine_jsx_configuration_logic() {
    // Unit test for determine_jsx_configuration logic
    // This test verifies the v3 JSX configuration logic matches the v2 JSTransformer behaviour
    // See: packages/transformers/js/src/JSTransformer.ts

    use crate::package_json::DependencyList;
    use crate::ts_config::CompilerOptions;

    let transformer = AtlaspackJsTransformerPlugin::new(&PluginContext {
      config: Arc::new(ConfigLoader {
        fs: Arc::new(InMemoryFileSystem::default()),
        project_root: PathBuf::from("test"),
        search_path: PathBuf::from("test"),
      }),
      file_system: Arc::new(InMemoryFileSystem::default()),
      logger: PluginLogger::default(),
      options: Arc::new(PluginOptions::default()),
    })
    .unwrap();

    let asset = Asset {
      file_type: FileType::Js,
      is_source: true,
      env: Arc::new(Environment {
        context: EnvironmentContext::Browser,
        source_type: SourceType::Module,
        output_format: OutputFormat::EsModule,
        ..Default::default()
      }),
      ..Default::default()
    };

    // Case 1: .js file with tsconfig jsx: "react" and React dependency
    // Expected: is_jsx = true, pragmas set from React dependency
    let compiler_options = Some(CompilerOptions {
      jsx: Some(Jsx::React),
      ..Default::default()
    });

    let package_json_with_react = Some(ConfigFile {
      contents: PackageJson {
        dependencies: Some(DependencyList {
          react: Some("^18.0.0".to_string()),
        }),
        ..Default::default()
      },
      path: PathBuf::from("package.json"),
      raw: r#"{ "dependencies": { "react": "^18.0.0" } }"#.to_string(),
    });

    let (is_jsx, jsx_pragma, jsx_pragma_frag, _, _) = transformer.determine_jsx_configuration(
      &FileType::Js,
      &asset,
      compiler_options.as_ref(),
      &package_json_with_react,
    );

    assert!(
      is_jsx,
      "JS file with tsconfig jsx: 'react' should have JSX enabled"
    );
    assert_eq!(jsx_pragma, Some("React.createElement".to_string()));
    assert_eq!(jsx_pragma_frag, Some("React.Fragment".to_string()));

    // Case 2: .js file with NO tsconfig and NO React dependency
    // Expected: is_jsx = false, no pragmas
    let package_json_no_react = Some(ConfigFile {
      contents: PackageJson {
        dependencies: Some(DependencyList { react: None }),
        ..Default::default()
      },
      path: PathBuf::from("package.json"),
      raw: r#"{ "dependencies": {} }"#.to_string(),
    });

    let (is_jsx_no_config, jsx_pragma_no_config, jsx_pragma_frag_no_config, _, _) =
      transformer.determine_jsx_configuration(&FileType::Js, &asset, None, &package_json_no_react);

    assert!(
      !is_jsx_no_config,
      "JS file without tsconfig and without React should NOT have JSX enabled"
    );
    assert_eq!(jsx_pragma_no_config, None);
    assert_eq!(jsx_pragma_frag_no_config, None);

    // Case 3: .js file with React dependency but NO tsconfig
    // Expected: is_jsx = true, pragmas set from React dependency
    let (is_jsx_react_only, jsx_pragma_react_only, jsx_pragma_frag_react_only, _, _) = transformer
      .determine_jsx_configuration(&FileType::Js, &asset, None, &package_json_with_react);

    assert!(
      is_jsx_react_only,
      "JS file with React dependency should have JSX enabled even without tsconfig"
    );
    assert_eq!(
      jsx_pragma_react_only,
      Some("React.createElement".to_string())
    );
    assert_eq!(
      jsx_pragma_frag_react_only,
      Some("React.Fragment".to_string())
    );

    // Case 4: .jsx file should always have JSX enabled regardless of configuration
    // Expected: is_jsx = true, pragmas set from React dependency if present
    let (is_jsx_jsx_file, jsx_pragma_jsx_file, jsx_pragma_frag_jsx_file, _, _) =
      transformer.determine_jsx_configuration(&FileType::Jsx, &asset, None, &package_json_no_react);

    assert!(
      is_jsx_jsx_file,
      ".jsx files should always have JSX enabled (file extension check)"
    );
    // Note: .jsx files get JSX enabled by file extension, but pragmas are only set if React is present
    assert_eq!(jsx_pragma_jsx_file, None);
    assert_eq!(jsx_pragma_frag_jsx_file, None);

    // Case 5: .jsx file with React dependency should have pragmas set
    let (is_jsx_jsx_with_react, jsx_pragma_jsx_with_react, jsx_pragma_frag_jsx_with_react, _, _) =
      transformer.determine_jsx_configuration(
        &FileType::Jsx,
        &asset,
        None,
        &package_json_with_react,
      );

    assert!(
      is_jsx_jsx_with_react,
      ".jsx files should always have JSX enabled"
    );
    assert_eq!(
      jsx_pragma_jsx_with_react,
      Some("React.createElement".to_string())
    );
    assert_eq!(
      jsx_pragma_frag_jsx_with_react,
      Some("React.Fragment".to_string())
    );

    // Case 6: .ts file should never have JSX enabled
    // Expected: is_jsx = false regardless of configuration
    let ts_asset = Asset {
      file_type: FileType::Ts,
      is_source: true,
      env: asset.env.clone(),
      ..Default::default()
    };

    let (is_jsx_ts, jsx_pragma_ts, jsx_pragma_frag_ts, _, _) = transformer
      .determine_jsx_configuration(
        &FileType::Ts,
        &ts_asset,
        compiler_options.as_ref(),
        &package_json_with_react,
      );

    assert!(
      !is_jsx_ts,
      ".ts files should never have JSX enabled (explicitly disabled)"
    );
    // Pragmas might be set if React is present, but JSX won't be enabled
    assert_eq!(jsx_pragma_ts, Some("React.createElement".to_string()));
    assert_eq!(jsx_pragma_frag_ts, Some("React.Fragment".to_string()));

    // Case 7: .tsx file should always have JSX enabled
    // Expected: is_jsx = true, pragmas set from React dependency if present
    let (is_jsx_tsx, jsx_pragma_tsx, jsx_pragma_frag_tsx, _, _) = transformer
      .determine_jsx_configuration(&FileType::Tsx, &asset, None, &package_json_with_react);

    assert!(
      is_jsx_tsx,
      ".tsx files should always have JSX enabled (file extension check)"
    );
    assert_eq!(jsx_pragma_tsx, Some("React.createElement".to_string()));
    assert_eq!(jsx_pragma_frag_tsx, Some("React.Fragment".to_string()));
  }

  // End-to-end tests for the new determine_jsx_configuration logic
  // These tests verify that the JSX configuration works correctly in the full transform pipeline

  mod jsx_configuration {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn js_file_with_tsconfig_and_react() -> anyhow::Result<()> {
      // Case 1: .js file with React dependency and tsconfig jsx: "react"
      // Expected: JSX should be enabled and transformed
      let file_system = Arc::new(InMemoryFileSystem::default());

      let target_asset = create_asset(
        "render.js",
        "
          import React from 'react';

          const Component = () => {
            return <div>Hello World</div>;
          };

          export default Component;
        ",
      );

      // Set up package.json with React dependency
      file_system.write_file(
        Path::new("package.json"),
        r#"{ "dependencies": { "react": "^18.0.0" } }"#.to_string(),
      );

      // Set up tsconfig.json with JSX configuration
      file_system.write_file(
        Path::new("tsconfig.json"),
        r#"{
          "compilerOptions": {
            "jsx": "react",
            "target": "es2015"
          }
        }"#
          .to_string(),
      );

      let result = run_test(TestOptions {
        asset: target_asset.clone(),
        file_system: Some(file_system),
        ..TestOptions::default()
      })
      .await;
      let transform_result = result?;
      let code = transform_result.asset.code.as_str()?;

      // JSX should be transformed
      assert!(
        !code.contains("<div"),
        "JSX should be transformed. Code: {}",
        code
      );
      // JSX should be transformed to either React.createElement or jsxDEV calls
      assert!(
        code.contains("createElement") || code.contains("jsxDEV") || code.contains("jsx"),
        "JSX should be transformed to React calls. Code: {}",
        code
      );
      // Verify dependencies include React
      let dependencies = get_dependencies(&transform_result);
      assert!(
        dependencies.contains(&"react".to_string()),
        "Dependencies should include 'react', but got: {:?}",
        dependencies
      );

      Ok(())
    }

    // v3JsxConfigurationLoading CLEANUP NOTE: Remove this test after rollout.
    #[tokio::test(flavor = "multi_thread")]
    async fn js_file_with_tsconfig_and_react_feature_flag_off() -> anyhow::Result<()> {
      // Case 1: .js file with React dependency and tsconfig jsx: "react"
      // Expected: JSX parsing should FAIL because .js files don't have JSX enabled in old logic
      let file_system = Arc::new(InMemoryFileSystem::default());

      let target_asset = create_asset(
        "render.js",
        "
          import React from 'react';

          const Component = () => {
            return <div>Hello World</div>;
          };

          export default Component;
        ",
      );

      // Set up package.json with React dependency
      file_system.write_file(
        Path::new("package.json"),
        r#"{ "dependencies": { "react": "^18.0.0" } }"#.to_string(),
      );

      // Set up tsconfig.json with JSX configuration
      file_system.write_file(
        Path::new("tsconfig.json"),
        r#"{
          "compilerOptions": {
            "jsx": "react",
            "target": "es2015"
          }
        }"#
          .to_string(),
      );

      // Test with feature flag OFF (old behaviour)
      let result = run_test(TestOptions {
        asset: target_asset.clone(),
        file_system: Some(file_system.clone()),
        feature_flags: Some(FeatureFlags::with_bool_flag(
          "v3JsxConfigurationLoading",
          false,
        )),
        ..TestOptions::default()
      })
      .await;

      // Old behaviour: .js files should NOT have JSX enabled even with React dependency
      // This should result in a parsing error since JSX is not enabled
      assert!(
        result.is_err(),
        "Old behaviour: .js files with JSX should fail to parse when JSX is not enabled. Error: {:?}",
        result.err()
      );

      Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn js_file_without_react_or_tsconfig() -> anyhow::Result<()> {
      // Case 2: .js file with NO React dependency and NO tsconfig
      // Expected: JSX parsing should FAIL
      let file_system = Arc::new(InMemoryFileSystem::default());

      let target_asset = create_asset(
        "render.js",
        "
          const Component = () => {
            return <div>Hello World</div>;
          };

          export default Component;
        ",
      );

      // Set up package.json WITHOUT React dependency
      file_system.write_file(
        Path::new("package.json"),
        r#"{ "dependencies": {} }"#.to_string(),
      );

      let result = run_test(TestOptions {
        asset: target_asset.clone(),
        file_system: Some(file_system),
        ..TestOptions::default()
      })
      .await;

      // JSX parsing should fail
      assert!(
        result.is_err(),
        "JSX parsing should fail when no React dependency and no tsconfig. Error: {:?}",
        result.err()
      );

      Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn js_file_with_react_only() -> anyhow::Result<()> {
      // Case 3: .js file with React dependency but NO tsconfig
      // Expected: JSX should be enabled and transformed
      let file_system = Arc::new(InMemoryFileSystem::default());

      let target_asset = create_asset(
        "render.js",
        "
          import React from 'react';

          const Component = () => {
            return <div>Hello World</div>;
          };

          export default Component;
        ",
      );

      // Set up package.json with React dependency
      file_system.write_file(
        Path::new("package.json"),
        r#"{ "dependencies": { "react": "^18.0.0" } }"#.to_string(),
      );

      let result = run_test(TestOptions {
        asset: target_asset.clone(),
        file_system: Some(file_system),
        ..TestOptions::default()
      })
      .await;
      let transform_result = result?;
      let code = transform_result.asset.code.as_str()?;

      // JSX should be transformed
      assert!(
        !code.contains("<div"),
        "JSX should be transformed with React dependency. Code: {}",
        code
      );
      // JSX should be transformed to either React.createElement or jsxDEV calls
      assert!(
        code.contains("createElement") || code.contains("jsxDEV") || code.contains("jsx"),
        "JSX should be transformed to React calls. Code: {}",
        code
      );
      // Verify dependencies include React
      let dependencies = get_dependencies(&transform_result);
      assert!(
        dependencies.contains(&"react".to_string()),
        "Dependencies should include 'react', but got: {:?}",
        dependencies
      );

      Ok(())
    }

    // v3JsxConfigurationLoading CLEANUP NOTE: Remove this test after rollout.
    #[tokio::test(flavor = "multi_thread")]
    async fn js_file_with_react_only_feature_flag_off() -> anyhow::Result<()> {
      // Case 3: .js file with React dependency but NO tsconfig
      // Expected: JSX parsing should FAIL because .js files don't have JSX enabled in old logic
      let file_system = Arc::new(InMemoryFileSystem::default());

      let target_asset = create_asset(
        "render.js",
        "
          import React from 'react';

          const Component = () => {
            return <div>Hello World</div>;
          };

          export default Component;
        ",
      );

      // Set up package.json with React dependency
      file_system.write_file(
        Path::new("package.json"),
        r#"{ "dependencies": { "react": "^18.0.0" } }"#.to_string(),
      );

      // Test with feature flag OFF (old behaviour)
      let result = run_test(TestOptions {
        asset: target_asset.clone(),
        file_system: Some(file_system.clone()),
        feature_flags: Some(FeatureFlags::with_bool_flag(
          "v3JsxConfigurationLoading",
          false,
        )),
        ..TestOptions::default()
      })
      .await;

      // Old behaviour: .js files should NOT have JSX enabled even with React dependency
      // This should result in a parsing error since JSX is not enabled
      assert!(
        result.is_err(),
        "Old behaviour: .js files with JSX should fail to parse when JSX is not enabled. Error: {:?}",
        result.err()
      );

      Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn jsx_file_without_react() -> anyhow::Result<()> {
      // Case 4: .jsx file with NO React dependency
      // Expected: JSX should be enabled but no pragmas (should still parse)
      let file_system = Arc::new(InMemoryFileSystem::default());

      let target_asset = create_asset(
        "render.jsx",
        "
          const Component = () => {
            return <div>Hello World</div>;
          };

          export default Component;
        ",
      );

      // Set up package.json WITHOUT React dependency
      file_system.write_file(
        Path::new("package.json"),
        r#"{ "dependencies": {} }"#.to_string(),
      );

      let result = run_test(TestOptions {
        asset: target_asset.clone(),
        file_system: Some(file_system),
        ..TestOptions::default()
      })
      .await;
      let transform_result = result?;
      let code = transform_result.asset.code.as_str()?;

      // JSX should be transformed (file extension enables JSX)
      assert!(
        !code.contains("<div"),
        "JSX should be transformed in .jsx files. Code: {}",
        code
      );
      // JSX should be transformed to either React.createElement or jsxDEV calls
      assert!(
        code.contains("createElement") || code.contains("jsxDEV") || code.contains("jsx"),
        "JSX should be transformed to React calls. Code: {}",
        code
      );

      Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn jsx_file_with_react() -> anyhow::Result<()> {
      // Case 5: .jsx file with React dependency
      // Expected: JSX should be enabled with React pragmas
      let file_system = Arc::new(InMemoryFileSystem::default());

      let target_asset = create_asset(
        "render.jsx",
        "
          import React from 'react';

          const Component = () => {
            return <div>Hello World</div>;
          };

          export default Component;
        ",
      );

      // Set up package.json with React dependency
      file_system.write_file(
        Path::new("package.json"),
        r#"{ "dependencies": { "react": "^18.0.0" } }"#.to_string(),
      );

      let result = run_test(TestOptions {
        asset: target_asset.clone(),
        file_system: Some(file_system),
        ..TestOptions::default()
      })
      .await;
      let transform_result = result?;
      let code = transform_result.asset.code.as_str()?;

      // JSX should be transformed with React pragmas
      assert!(
        !code.contains("<div"),
        "JSX should be transformed in .jsx files with React. Code: {}",
        code
      );
      // JSX should be transformed to either React.createElement or jsxDEV calls
      assert!(
        code.contains("createElement") || code.contains("jsxDEV") || code.contains("jsx"),
        "JSX should be transformed to React calls. Code: {}",
        code
      );
      // Verify dependencies include React
      let dependencies = get_dependencies(&transform_result);
      assert!(
        dependencies.contains(&"react".to_string()),
        "Dependencies should include 'react', but got: {:?}",
        dependencies
      );

      Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ts_file_never_has_jsx() -> anyhow::Result<()> {
      // Case 6: .ts file with React dependency and tsconfig jsx: "react"
      // Expected: JSX parsing should FAIL (TypeScript files never have JSX)

      let file_system = Arc::new(InMemoryFileSystem::default());

      let target_asset = create_asset(
        "render.ts",
        "
          import React from 'react';

          const Component = () => {
            return <div>Hello World</div>;
          };

          export default Component;
        ",
      );

      // Set up package.json with React dependency
      file_system.write_file(
        Path::new("package.json"),
        r#"{ "dependencies": { "react": "^18.0.0" } }"#.to_string(),
      );

      // Set up tsconfig.json with JSX configuration
      file_system.write_file(
        Path::new("tsconfig.json"),
        r#"{
          "compilerOptions": {
            "jsx": "react",
            "target": "es2015"
          }
        }"#
          .to_string(),
      );

      let result = run_test(TestOptions {
        asset: target_asset.clone(),
        file_system: Some(file_system),
        ..TestOptions::default()
      })
      .await;

      // JSX parsing should fail (TypeScript files never have JSX)
      assert!(
        result.is_err(),
        "JSX parsing should fail in .ts files even with React dependency and tsconfig. Error: {:?}",
        result.err()
      );

      Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn tsx_file_with_react() -> anyhow::Result<()> {
      // Case 7: .tsx file with React dependency
      // Expected: JSX should be enabled with React pragmas
      let file_system = Arc::new(InMemoryFileSystem::default());

      let target_asset = create_asset(
        "render.tsx",
        "
          import React from 'react';

          const Component = () => {
            return <div>Hello World</div>;
          };

          export default Component;
        ",
      );

      // Set up package.json with React dependency
      file_system.write_file(
        Path::new("package.json"),
        r#"{ "dependencies": { "react": "^18.0.0" } }"#.to_string(),
      );

      let result = run_test(TestOptions {
        asset: target_asset.clone(),
        file_system: Some(file_system),
        ..TestOptions::default()
      })
      .await;
      let transform_result = result?;
      let code = transform_result.asset.code.as_str()?;

      // JSX should be transformed with React pragmas
      assert!(
        !code.contains("<div"),
        "JSX should be transformed in .tsx files with React. Code: {}",
        code
      );
      // JSX should be transformed to either React.createElement or jsxDEV calls
      assert!(
        code.contains("createElement") || code.contains("jsxDEV") || code.contains("jsx"),
        "JSX should be transformed to React calls. Code: {}",
        code
      );
      // Verify dependencies include React
      let dependencies = get_dependencies(&transform_result);
      assert!(
        dependencies.contains(&"react".to_string()),
        "Dependencies should include 'react', but got: {:?}",
        dependencies
      );

      Ok(())
    }
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
          packaging_id: Some(target_asset.id.to_string()),
          ..target_asset
        },
        discovered_assets: vec![],
        dependencies: vec![],
        invalidate_on_file_change: vec![],
        cache_bailout: false,
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

    let expected_dependencies = vec![
      DependencyBuilder::default()
        .loc(SourceLocation {
          file_path: PathBuf::from("mock_path.js"),
          start: Location {
            line: 2,
            column: 25,
          },
          end: Location {
            line: 2,
            column: 32,
          },
        })
        .placeholder("e83f3db3d6f57ea6".to_string())
        .kind(DependencyKind::Require)
        .source_asset_id(asset_id.clone())
        .source_path(PathBuf::from("mock_path.js"))
        .source_asset_type(FileType::Js)
        .specifier(String::from("other"))
        .specifier_type(SpecifierType::CommonJS)
        .symbols(vec![Symbol {
          exported: String::from("*"),
          loc: None,
          local: String::from("a1ad9714284f3ad6$"),
          ..Symbol::default()
        }])
        .env(Arc::new(Environment::default()))
        .priority(Priority::Sync)
        .build(),
    ];

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
          packaging_id: Some(asset_id.to_string()),
          ..empty_asset()
        },
        discovered_assets: vec![],
        dependencies: expected_dependencies,
        invalidate_on_file_change: vec![],
        cache_bailout: false,
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
      false,
      env.clone(),
      file_path.into(),
      None,
      &PathBuf::default(),
      None,
      false,
    )
    .unwrap()
  }

  fn create_asset_at_project_root(project_root: &Path, file_path: &str, code: &str) -> Asset {
    let env = Arc::new(Environment::default());

    Asset::new(
      Code::from(code),
      false,
      env.clone(),
      file_path.into(),
      None,
      project_root,
      None,
      false,
    )
    .unwrap()
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
    feature_flags: Option<FeatureFlags>,
  }

  async fn run_test(options: TestOptions) -> anyhow::Result<TransformResult> {
    let asset = options.asset;
    let project_root = options.project_root.unwrap_or_default();
    let file_system = options
      .file_system
      .unwrap_or_else(|| default_fs(&project_root));

    // v3JsxConfigurationLoading CLEANUP NOTE: Remove 'flag enabled for tests by default' logic
    let plugin_options = PluginOptions {
      feature_flags: options
        .feature_flags
        .unwrap_or_default()
        .with_bool_flag_default("v3JsxConfigurationLoading", true),
      ..PluginOptions::default()
    };

    let ctx = PluginContext {
      config: Arc::new(ConfigLoader {
        fs: file_system.clone(),
        project_root: project_root.to_path_buf(),
        search_path: project_root.to_path_buf(),
      }),
      file_system: file_system.clone(),
      logger: PluginLogger::default(),
      options: Arc::new(plugin_options),
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

    result.asset.code = Code::from(normalize_code(result.asset.code.as_str()?));

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
