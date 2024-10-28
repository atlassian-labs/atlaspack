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
use serde::Deserialize;
use swc_core::atoms::Atom;

use crate::js_transformer_config::{InlineEnvironment, JsTransformerConfig};
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

#[derive(Deserialize)]
struct PackageJson {
  #[serde(rename = "@atlaspack/transformer-js")]
  config: Option<JsTransformerConfig>,
}

impl AtlaspackJsTransformerPlugin {
  pub fn new(ctx: &PluginContext) -> Result<Self, Error> {
    let config = ctx.config.load_package_json::<PackageJson>().map_or_else(
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
  /// This does a lot of equivalent work to `JSTransformer::transform` in
  /// `packages/transformers/js`
  async fn transform(
    &self,
    _context: TransformContext,
    asset: Asset,
  ) -> Result<TransformResult, Error> {
    let env = asset.env.clone();
    let file_type = asset.file_type.clone();
    let is_node = env.context.is_node();
    let source_code = asset.code.clone();

    let mut targets: HashMap<String, String> = HashMap::new();
    if env.context.is_browser() {
      let browsers = env.engines.browsers.clone().unwrap_or_default();
      let browsers = Browsers::from(browsers);
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

    let transformation_result = atlaspack_js_swc_core::transform(
      atlaspack_js_swc_core::Config {
        // TODO: Infer from package.json
        automatic_jsx_runtime: compiler_options
          .map(|co| {
            co.jsx
              .as_ref()
              .is_some_and(|jsx| matches!(jsx, Jsx::ReactJsx | Jsx::ReactJsxDev))
              || co.jsx_import_source.is_some()
          })
          .unwrap_or_default(),
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
        // TODO Infer from package.json
        jsx_import_source: compiler_options.and_then(|co| co.jsx_import_source.clone()),
        jsx_pragma: compiler_options.and_then(|co| co.jsx_factory.clone()),
        jsx_pragma_frag: compiler_options.and_then(|co| co.jsx_fragment_factory.clone()),
        module_id: asset.id.to_string(),
        node_replacer: is_node,
        project_root: self.options.project_root.to_string_lossy().into_owned(),
        // TODO: Boolean(
        //   pkg?.dependencies?.react ||
        //     pkg?.devDependencies?.react ||
        //     pkg?.peerDependencies?.react,
        // );
        react_refresh: self.options.mode == BuildMode::Development
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
        // TODO: Update transformer to use engines directly
        targets: Some(targets),
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
      },
      None,
    )?;

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
  use pretty_assertions::assert_eq;
  use std::path::{Path, PathBuf};

  use atlaspack_core::{
    config_loader::ConfigLoader,
    plugin::PluginLogger,
    types::{Code, Dependency, Environment, Location, SourceLocation, SpecifierType, Symbol},
  };
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;

  use super::*;

  fn empty_asset() -> Asset {
    Asset {
      file_type: FileType::Js,
      is_bundle_splittable: true,
      is_source: true,
      ..Default::default()
    }
  }

  fn create_asset(project_root: &Path, file_path: &str, code: &str) -> Asset {
    let file_system = Arc::new(InMemoryFileSystem::default());
    let env = Arc::new(Environment::default());

    Asset::new(
      project_root,
      env.clone(),
      file_path.into(),
      Some(String::from(code)),
      None,
      false,
      None,
      file_system.clone(),
    )
    .unwrap()
  }

  #[test]
  fn test_asset_id_is_stable() {
    let project_root = Path::new("/root");
    let asset_1 = create_asset(project_root, "mock_path", "function hello() {}");
    let asset_2 = create_asset(project_root, "mock_path", "function helloButDifferent() {}");

    // This nº should not change across runs / compilation
    assert_eq!(asset_1.id, "4711cac63cb78f2f");
    assert_eq!(asset_1.id, asset_2.id);
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_transformer_on_noop_asset() {
    let project_root = Path::new("/root");
    let target_asset = create_asset(project_root, "mock_path.js", "function hello() {}");
    let result = run_test(target_asset.clone()).await.unwrap();

    assert_eq!(
      result,
      TransformResult {
        asset: Asset {
          file_path: "mock_path.js".into(),
          file_type: FileType::Js,
          // SWC inserts a newline here
          code: Box::new(Code::from(String::from("function hello() {}\n"))),
          symbols: Some(Vec::new()),
          unique_key: None,
          ..target_asset
        },
        discovered_assets: vec![],
        dependencies: vec![],
        invalidate_on_file_change: vec![]
      }
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_transformer_on_asset_that_requires_other() {
    let source_code = r#"
      const x = require('other');
      exports.hello = function() {};
    "#;

    let project_root = Path::new("/root");
    let target_asset = create_asset(project_root, "mock_path.js", source_code);
    let asset_id = target_asset.id.clone();
    let result = run_test(target_asset).await.unwrap();

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

    assert_eq!(result.dependencies, expected_dependencies);
    assert_eq!(
      result,
      TransformResult {
        asset: Asset {
          id: asset_id.clone(),
          file_path: "mock_path.js".into(),
          file_type: FileType::Js,
          // SWC inserts a newline here
          code: Box::new(Code::from(String::from(
            "var x = require(\"e83f3db3d6f57ea6\");\nexports.hello = function() {};\n"
          ))),
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
          ..empty_asset()
        },
        discovered_assets: vec![],
        dependencies: expected_dependencies,
        invalidate_on_file_change: vec![]
      }
    );
  }

  async fn run_test(asset: Asset) -> anyhow::Result<TransformResult> {
    let file_system = Arc::new(InMemoryFileSystem::default());
    let project_root = PathBuf::default();

    file_system.write_file(&project_root.join("package.json"), String::from("{}"));

    let ctx = PluginContext {
      config: Arc::new(ConfigLoader {
        fs: file_system.clone(),
        project_root: project_root.clone(),
        search_path: project_root.clone(),
      }),
      file_system,
      logger: PluginLogger::default(),
      options: Arc::new(PluginOptions::default()),
    };

    let transformer = AtlaspackJsTransformerPlugin::new(&ctx).expect("Expected transformer");
    let context = TransformContext::default();

    let result = transformer.transform(context, asset).await?;
    Ok(result)
  }
}
