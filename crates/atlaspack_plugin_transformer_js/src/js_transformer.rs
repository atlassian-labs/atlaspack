use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::PathBuf;

use anyhow::{Error, anyhow};

use async_trait::async_trait;
use atlaspack_core::config_loader::ConfigLoaderRef;
use atlaspack_core::define_feature_flags;
use atlaspack_core::plugin::TransformResult;
use atlaspack_core::plugin::{HmrOptions, PluginContext, TransformerPlugin};
use atlaspack_core::types::browsers::Browsers;
use atlaspack_core::types::engines::EnvironmentFeature;
use atlaspack_core::types::{
  Asset, BuildMode, DependencyKind, Diagnostic, Diagnostics, ErrorKind, FileType, OutputFormat,
  SourceType,
};
use atlaspack_js_swc_core::SyncDynamicImportConfig;
use derivative::Derivative;
use glob_match::glob_match;
use swc_core::atoms::Atom;

use crate::js_transformer_config::{
  InlineEnvironment, JsTransformerConfig, JsTransformerPackageJson,
};
use crate::map_diagnostics::{MapDiagnosticOptions, map_diagnostics};
use crate::package_json::{PackageJson, depends_on_react, supports_automatic_jsx_runtime};
use crate::ts_config::{Jsx, Target, TsConfig};

pub use atlaspack_js_swc_core::JsxConfiguration;

mod conversion;

// Define the feature flags struct for this transformer
define_feature_flags!(JsTransformerFlags, {
  conditionalBundlingApi,
  exportsRebindingOptimisation,
  hmrImprovements,
  nestedPromiseImportFix,
  newJsxConfig
});

#[derive(Clone, Hash)]
struct EnvFeatures {
  enable_ssr_typeof_replacement: bool,
  global_aliasing_config: Option<BTreeMap<String, String>>,
  enable_lazy_loading: bool,
  enable_react_hooks_removal: bool,
  enable_react_async_import_lift: bool,
  react_async_lift_by_default: bool,
  react_async_lift_report_level: String,
  enable_static_prevaluation: bool,
  enable_dead_returns_removal: bool,
  enable_unused_bindings_removal: bool,
  sync_dynamic_import_config: Option<SyncDynamicImportConfig>,
}

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
#[derive(Derivative)]
#[derivative(Hash)]
pub struct AtlaspackJsTransformerPlugin {
  config: JsTransformerConfig,
  project_root: PathBuf,
  mode: BuildMode,
  source_asset_env: BTreeMap<Atom, Atom>,
  external_asset_env: BTreeMap<Atom, Atom>,
  hmr_options: Option<HmrOptions>,
  feature_flags: JsTransformerFlags,
  core_path: PathBuf,
  ts_config: Option<TsConfig>,
  env_features: EnvFeatures,
  // We can ignore this from cache because when it's used we trigger a cache
  // bailout. Should be removed once we clean-up the "newJsxConfig" feature flag.
  #[derivative(Hash = "ignore")]
  config_loader: ConfigLoaderRef,
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

    // Filter environment variables based on inline_environment config to only include
    // what's needed in the cache key. This prevents unnecessary cache invalidation.
    // Note: We have two envs, one for source assets and one for external
    // assets. For assets with custom_env, the filtering will be done on-demand in env_variables().
    let source_asset_env = Self::filter_env_vars(
      &ctx.options.env,
      config
        .inline_environment
        .as_ref()
        .unwrap_or(&InlineEnvironment::Enabled(true)),
    );
    let external_asset_env = Self::filter_env_vars(
      &ctx.options.env,
      config
        .inline_environment
        .as_ref()
        .unwrap_or(&InlineEnvironment::Enabled(false)),
    );

    let env_features = Self::get_env_features(&ctx.options.env);

    let core_path = ctx.options.core_path.clone();
    let hmr_options = ctx.options.hmr_options.clone();
    let mode = ctx.options.mode.clone();
    let project_root = ctx.options.project_root.clone();

    let feature_flags = JsTransformerFlags::new(&ctx.options.feature_flags);

    Ok(Self {
      config,
      core_path,
      source_asset_env,
      external_asset_env,
      env_features,
      feature_flags,
      hmr_options,
      mode,
      project_root,
      ts_config,
      config_loader: ctx.config.clone(),
    })
  }

  /// Determines JSX configuration based on file type, tsconfig, and package.json
  fn determine_jsx_configuration(&self, asset: &Asset) -> JsxConfiguration {
    let file_type = match &asset.file_type {
      FileType::Js => "js",
      FileType::Jsx => "jsx",
      FileType::Ts => "ts",
      FileType::Tsx => "tsx",
      _ => "unknown",
    };

    atlaspack_js_swc_core::determine_jsx_configuration(
      &asset.file_path,
      file_type,
      asset.is_source,
      &self.config.jsx,
      &self.project_root,
    )
  }
}

impl AtlaspackJsTransformerPlugin {
  /// Filters a set of environment variables based on the inline_environment configuration.
  /// This is a shared helper used both for cache key generation and runtime filtering.
  fn filter_env_vars(
    env_vars: &BTreeMap<String, String>,
    inline_environment: &InlineEnvironment,
  ) -> BTreeMap<Atom, Atom> {
    match inline_environment {
      InlineEnvironment::Enabled(false) => {
        // Only include NODE_ENV and ATLASPACK_BUILD_ENV when disabled
        let mut filtered = BTreeMap::new();
        if let Some(node_env) = env_vars.get("NODE_ENV") {
          filtered.insert("NODE_ENV".into(), node_env.clone().into());
        }
        if let Some(build_env) = env_vars.get("ATLASPACK_BUILD_ENV")
          && build_env == "test"
        {
          filtered.insert("ATLASPACK_BUILD_ENV".into(), build_env.clone().into());
        }
        filtered
      }
      InlineEnvironment::Enabled(true) => {
        // Include all env vars
        let mut filtered = BTreeMap::new();
        for (key, value) in env_vars.iter() {
          filtered.insert(key.clone().into(), value.clone().into());
        }
        filtered
      }
      InlineEnvironment::Environments(environments) => {
        // Filter based on glob patterns
        let mut filtered = BTreeMap::new();
        for env_glob in environments {
          for (key, value) in env_vars.iter() {
            if glob_match(env_glob, key) {
              filtered.insert(key.clone().into(), value.clone().into());
            }
          }
        }
        filtered
      }
    }
  }

  fn env_variables(&self, asset: &Asset) -> BTreeMap<Atom, Atom> {
    let env = if asset.is_source {
      &self.source_asset_env
    } else {
      &self.external_asset_env
    };

    if let Some(custom_env) = &asset.env.custom_env {
      let mut filtered_custom_env = Self::filter_env_vars(
        custom_env,
        self
          .config
          .inline_environment
          .as_ref()
          .unwrap_or(&InlineEnvironment::Enabled(asset.is_source)),
      );

      filtered_custom_env.extend(env.clone());

      return filtered_custom_env;
    }

    env.clone()
  }

  fn get_env_features(env_vars: &BTreeMap<String, String>) -> EnvFeatures {
    let enable_ssr_typeof_replacement = env_vars
      .get("NATIVE_SSR_TYPEOF_REPLACEMENT")
      .is_some_and(|v| v == "true");
    let global_aliasing_config = match env_vars.get("NATIVE_GLOBAL_ALIASING") {
      Some(value) => match serde_json::from_str::<BTreeMap<String, String>>(value) {
        Ok(config) => Some(config),
        Err(err) => {
          eprintln!(
            "Failed to parse NATIVE_GLOBAL_ALIASING JSON: {}. Config will not be applied.",
            err
          );
          None
        }
      },
      None => None,
    };
    let enable_lazy_loading = env_vars
      .get("NATIVE_LAZY_LOADING")
      .is_some_and(|v| v == "true");
    let enable_react_hooks_removal = env_vars
      .get("NATIVE_REACT_HOOKS_REMOVAL")
      .is_some_and(|v| v == "true");
    let enable_react_async_import_lift = env_vars
      .get("NATIVE_REACT_ASYNC_IMPORT_LIFT")
      .is_some_and(|v| v == "true");
    let react_async_lift_by_default = env_vars
      .get("REACT_ASYNC_IMPORT_LIFTING_BY_DEFAULT")
      .is_some_and(|v| v == "true");
    let react_async_lift_report_level = env_vars
      .get("REACT_ASYNC_LIFT_REPORT_LEVEL")
      .cloned()
      .unwrap_or_else(|| String::from("none"));
    let enable_static_prevaluation = env_vars
      .get("NATIVE_PREVALUATION")
      .is_some_and(|v| v == "true");
    let enable_dead_returns_removal = env_vars
      .get("NATIVE_DEAD_RETURNS_REMOVAL")
      .is_some_and(|v| v == "true");
    let enable_unused_bindings_removal = env_vars
      .get("NATIVE_UNUSED_BINDINGS_REMOVAL")
      .is_some_and(|v| v == "true");

    let sync_dynamic_import_config = if let Some(sync_dynamic_import_config) =
      env_vars.get("SYNC_DYNAMIC_IMPORT_CONFIG")
    {
      match serde_json::from_str::<SyncDynamicImportConfig>(sync_dynamic_import_config) {
        Ok(config) => Some(config),
        Err(_) => {
          tracing::error!(
            "Failed to parse SYNC_DYNAMIC_IMPORT_CONFIG to JSON or config shape did not match. Config will not be applied."
          );

          let fallback = SyncDynamicImportConfig {
            entrypoint_filepath_suffix: "__NO_MATCH__".into(),
            actual_require_paths: vec![],
          };

          Some(fallback)
        }
      }
    } else {
      None
    };

    EnvFeatures {
      enable_ssr_typeof_replacement,
      global_aliasing_config,
      enable_lazy_loading,
      enable_react_hooks_removal,
      enable_react_async_import_lift,
      react_async_lift_by_default,
      react_async_lift_report_level,
      enable_static_prevaluation,
      enable_dead_returns_removal,
      enable_unused_bindings_removal,
      sync_dynamic_import_config,
    }
  }
}

impl fmt::Debug for AtlaspackJsTransformerPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("AtlaspackJsTransformerPlugin").finish()
  }
}

#[async_trait]
impl TransformerPlugin for AtlaspackJsTransformerPlugin {
  /// This does equivalent work to `JSTransformer::transform` in `packages/transformers/js`
  async fn transform(&self, asset: Asset) -> Result<TransformResult, Error> {
    let env = asset.env.clone();
    let is_node = env.context.is_node();
    let source_code = asset.code.clone();
    let mut cache_bailout = false;

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

    let EnvFeatures {
      enable_ssr_typeof_replacement,
      global_aliasing_config,
      enable_lazy_loading,
      enable_react_hooks_removal,
      enable_react_async_import_lift,
      react_async_lift_by_default,
      react_async_lift_report_level,
      enable_static_prevaluation,
      enable_dead_returns_removal,
      enable_unused_bindings_removal,
      mut sync_dynamic_import_config,
    } = self.env_features.clone();

    if !env.context.is_tesseract() {
      sync_dynamic_import_config = None;
    }

    let compiler_options = self
      .ts_config
      .as_ref()
      .and_then(|ts| ts.compiler_options.as_ref());

    // Determine JSX configuration based on newJsxConfig feature flag
    let JsxConfiguration {
      is_jsx,
      jsx_pragma,
      jsx_pragma_frag,
      jsx_import_source,
      automatic_jsx_runtime,
      react_refresh,
    } = if self.feature_flags.newJsxConfig() {
      self.determine_jsx_configuration(&asset)
    } else {
      // With newJsxConfig disabled, use the old logic
      let package_json = self
        .config_loader
        .load_local_package_json::<PackageJson>(&asset.file_path)
        .ok();
      let is_jsx = matches!(asset.file_type, FileType::Jsx | FileType::Tsx);

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
      let react_refresh = package_json.is_some_and(|pkg| depends_on_react(&pkg.contents));

      JsxConfiguration {
        is_jsx,
        jsx_pragma,
        jsx_pragma_frag,
        jsx_import_source,
        automatic_jsx_runtime,
        react_refresh,
      }
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
      is_development: self.mode == BuildMode::Development,
      is_esm_output: env.output_format == OutputFormat::EsModule,
      is_jsx,
      is_library: env.is_library,
      is_type_script: matches!(asset.file_type, FileType::Ts | FileType::Tsx),
      is_worker: env.context.is_worker(),
      jsx_import_source,
      jsx_pragma,
      jsx_pragma_frag,
      magic_comments: self.config.magic_comments.unwrap_or_default(),
      add_display_name: self.config.add_react_display_name,
      module_id: asset.id.to_string(),
      node_replacer: is_node,
      project_root: self.project_root.to_string_lossy().into_owned(),
      react_refresh: self.hmr_options.is_some()
        && self.mode == BuildMode::Development
        && react_refresh
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
      trace_bailouts: false, // Simplified: could be made configurable if needed
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
      conditional_bundling: self.feature_flags.conditionalBundlingApi(),
      hmr_improvements: self.feature_flags.hmrImprovements(),
      exports_rebinding_optimisation: self.feature_flags.exportsRebindingOptimisation(),
      nested_promise_import_fix: self.feature_flags.nestedPromiseImportFix(),
      enable_ssr_typeof_replacement,
      global_aliasing_config,
      enable_lazy_loading,
      enable_react_hooks_removal,
      enable_react_async_import_lift,
      react_async_lift_by_default,
      react_async_lift_report_level,
      enable_static_prevaluation,
      enable_dead_returns_removal,
      enable_unused_bindings_removal,
      sync_dynamic_import_config,
      is_swc_helpers: false,
      standalone: false,
    };

    let transformation_result = atlaspack_js_swc_core::transform(&transform_config, None)?;

    if transformation_result
      .dependencies
      .iter()
      .any(|dep| matches!(dep.kind, DependencyKind::File))
    {
      // If we've inlined FS call then the result is not cacheable
      tracing::info!(
        "Asset {} has inlined FS calls, marking transform result as non-cacheable",
        asset.file_path.display()
      );
      cache_bailout = true;
    }

    if let Some(errors) = transformation_result.diagnostics {
      return Err(anyhow!(map_diagnostics(
        errors,
        MapDiagnosticOptions {
          source_code: Some(source_code.clone()),
          file_type: Some(asset.file_type.clone()),
          file_path: Some(asset.file_path),
        },
      )));
    }

    let mut result = conversion::convert_result(
      asset,
      &transform_config,
      transformation_result,
      &self.project_root,
      &self.mode,
      &self.core_path,
      &self.hmr_options,
    )
    .map_err(|errors| anyhow!(Diagnostics::from(errors)))?;

    if cache_bailout {
      result.cache_bailout = cache_bailout;
    }

    Ok(result)
  }
}

#[cfg(test)]
mod tests {
  use std::{
    path::{Path, PathBuf},
    sync::Arc,
  };

  use atlaspack_core::{
    config_loader::ConfigLoader,
    plugin::{PluginLogger, PluginOptions},
    types::{
      Code, DependencyBuilder, DependencyKind, Environment, EnvironmentContext, FeatureFlags,
      Location, Priority, SourceLocation, SpecifierType, Symbol,
    },
  };
  use atlaspack_filesystem::{FileSystemRef, in_memory_file_system::InMemoryFileSystem};
  use atlaspack_js_swc_core::{AutomaticReactRuntime, AutomaticRuntimeGlobs, JsxOptions};
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
  async fn transforms_react_with_automatic_runtime_glob() -> anyhow::Result<()> {
    use atlaspack_test_fixtures::test_fixture;

    // Test automatic JSX runtime with glob pattern matching
    let project_root = PathBuf::from("/");
    let file_system = test_fixture! {
      project_root.clone(),
      "package.json" => r#"{
        "@atlaspack/transformer-js": {
          "jsx": {
            "pragma": "React.createElement",
            "pragmaFragment": "React.Fragment",
            "importSource": "react",
            "automaticRuntime": {
              "include": ["src/components/**/*.tsx"],
              "exclude": []
            }
          }
        }
      }"#,
      "src/components/Button.tsx" => r#"
        const Component = () => {
          return <div>Hello World</div>;
        };

        export default Component;
      "#
    };

    let target_asset = create_asset_at_project_root(
      &project_root,
      "/src/components/Button.tsx",
      r#"
        const Component = () => {
          return <div>Hello World</div>;
        };

        export default Component;
      "#,
    );

    let result = run_test(TestOptions {
      asset: target_asset,
      file_system: Some(file_system),
      project_root: Some(project_root),
      ..Default::default()
    })
    .await?;

    let code = result.asset.code.as_str()?;

    // With automatic runtime, JSX should be transformed to use jsx() imports
    // instead of React.createElement calls
    assert!(
      code.contains("jsx") || code.contains("_jsx"),
      "Code should use automatic JSX runtime (jsx imports), got: {}",
      code
    );

    // Should not contain React.createElement since we're using automatic runtime
    assert!(
      !code.contains("React.createElement"),
      "Code should not contain React.createElement with automatic runtime, got: {}",
      code
    );

    Ok(())
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn transforms_react_without_automatic_runtime_glob_no_match() -> anyhow::Result<()> {
    use atlaspack_test_fixtures::test_fixture;

    // Test that files not matching glob pattern don't get automatic runtime
    let project_root = PathBuf::from("/test");
    let file_system = test_fixture! {
      project_root.clone(),
      "package.json" => r#"{
        "@atlaspack/transformer-js": {
          "jsx": {
            "pragma": "React.createElement",
            "pragmaFragment": "React.Fragment",
            "importSource": "react",
            "automaticRuntime": {
              "include": ["src/components/**/*.tsx"],
              "exclude": []
            }
          }
        }
      }"#,
      "src/pages/Home.tsx" => r#"
        const Component = () => {
          return <div>Hello World</div>;
        };

        export default Component;
      "#
    };

    let target_asset = create_asset_at_project_root(
      &project_root,
      "src/pages/Home.tsx",
      r#"
        const Component = () => {
          return <div>Hello World</div>;
        };

        export default Component;
      "#,
    );

    let result = run_test(TestOptions {
      asset: target_asset,
      file_system: Some(file_system),
      project_root: Some(project_root),
      ..Default::default()
    })
    .await?;

    let code = result.asset.code.as_str()?;

    // Without automatic runtime matching, should use classic React.createElement
    assert!(
      code.contains("React.createElement") || code.contains("_react"),
      "Code should use classic JSX runtime (React.createElement), got: {}",
      code
    );

    Ok(())
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn transforms_react_with_automatic_runtime_glob_outside_project_root() -> anyhow::Result<()>
  {
    use atlaspack_test_fixtures::test_fixture;

    // Test automatic JSX runtime with glob pattern matching files outside project root
    // Asset is outside the project root: /dir/other-project/index.tsx
    // Project root is: /dir/my-project
    let project_root = PathBuf::from("/dir/my-project");
    let file_system = test_fixture! {
      project_root.clone(),
      "package.json" => r#"{
        "@atlaspack/transformer-js": {
          "jsx": {
            "pragma": "React.createElement",
            "pragmaFragment": "React.Fragment",
            "importSource": "react",
            "automaticRuntime": {
              "include": ["../other-project/**/*.tsx"],
              "exclude": []
            }
          }
        }
      }"#,
      "../other-project/index.tsx" => r#"
        const Component = () => {
          return <div>Outside Project Root</div>;
        };

        export default Component;
      "#
    };

    let target_asset = create_asset_at_project_root(
      &project_root,
      "/dir/other-project/index.tsx",
      r#"
        const Component = () => {
          return <div>Outside Project Root</div>;
        };

        export default Component;
      "#,
    );

    let result = run_test(TestOptions {
      asset: target_asset,
      file_system: Some(file_system),
      project_root: Some(project_root),
      ..Default::default()
    })
    .await?;

    let code = result.asset.code.as_str()?;

    // With automatic runtime, JSX should be transformed to use jsx() imports
    assert!(
      code.contains("jsx") || code.contains("_jsx"),
      "Code should use automatic JSX runtime (jsx imports) for files outside project root matching glob, got: {}",
      code
    );

    // Should not contain React.createElement since we're using automatic runtime
    assert!(
      !code.contains("React.createElement"),
      "Code should not contain React.createElement with automatic runtime, got: {}",
      code
    );

    Ok(())
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn transforms_react_without_automatic_runtime_glob_outside_project_root_no_match()
  -> anyhow::Result<()> {
    use atlaspack_test_fixtures::test_fixture;

    // Test that files outside project root that don't match glob use classic runtime
    // Asset is outside the project root: /dir/another-project/index.tsx
    // Project root is: /dir/my-project
    // Glob pattern: ../other-project/** should NOT match
    let project_root = PathBuf::from("/dir/my-project");
    let file_system = test_fixture! {
      project_root.clone(),
      "package.json" => r#"{
        "@atlaspack/transformer-js": {
          "jsx": {
            "pragma": "React.createElement",
            "pragmaFragment": "React.Fragment",
            "importSource": "react",
            "automaticRuntime": {
              "include": ["../other-project/**"],
              "exclude": []
            }
          }
        }
      }"#,
      "../another-project/index.tsx" => r#"
        const Component = () => {
          return <div>Different Outside Project</div>;
        };

        export default Component;
      "#
    };

    let target_asset = create_asset_at_project_root(
      &project_root,
      "/dir/another-project/index.tsx",
      r#"
        const Component = () => {
          return <div>Different Outside Project</div>;
        };

        export default Component;
      "#,
    );

    let result = run_test(TestOptions {
      asset: target_asset,
      file_system: Some(file_system),
      project_root: Some(project_root),
      ..Default::default()
    })
    .await?;

    let code = result.asset.code.as_str()?;

    // Without automatic runtime matching, should use classic React.createElement
    assert!(
      code.contains("React.createElement") || code.contains("_react"),
      "Code should use classic JSX runtime (React.createElement) for non-matching files outside project root, got: {}",
      code
    );

    Ok(())
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn transforms_react_with_inferred_automatic_runtime_from_package_json() -> anyhow::Result<()>
  {
    use atlaspack_test_fixtures::test_fixture;

    async fn test_version(version: &str) -> anyhow::Result<()> {
      let project_root = PathBuf::from("/test");
      let file_system = test_fixture! {
        project_root.clone(),
        "package.json" => &format!(r#"{{
          "dependencies": {{ "react": "{version}" }},
          "@atlaspack/transformer-js": {{
            "jsx": {{
              "pragma": "React.createElement",
              "pragmaFragment": "React.Fragment",
              "automaticRuntime": true
            }}
          }}
        }}"#),
        "index.jsx" => "const main = () => <div />;"
      };

      let target_asset =
        create_asset_at_project_root(&project_root, "index.jsx", "const main = () => <div />;");

      let result = run_test(TestOptions {
        asset: target_asset.clone(),
        file_system: Some(file_system),
        project_root: Some(project_root),
        ..Default::default()
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
    // This test verifies the newJsxConfig configuration logic using JsTransformerConfig
    // instead of the old package.json/tsconfig approach

    let mut transformer = AtlaspackJsTransformerPlugin::new(&PluginContext {
      config: Arc::new(ConfigLoader {
        fs: Arc::new(InMemoryFileSystem::default()),
        project_root: PathBuf::from("/"),
        search_path: PathBuf::from("/"),
      }),
      file_system: Arc::new(InMemoryFileSystem::default()),
      logger: PluginLogger::default(),
      options: Arc::new(PluginOptions {
        project_root: PathBuf::from("/"),
        mode: BuildMode::Development,
        env: BTreeMap::new(),
        hmr_options: None,
        core_path: PathBuf::from("test"),
        feature_flags: FeatureFlags::default()
          .with_bool_flag_default("conditionalBundlingApi", true)
          .with_bool_flag_default("hmrImprovements", true)
          .with_bool_flag_default("nestedPromiseImportFix", true)
          .with_bool_flag_default("exportsRebindingOptimisation", true)
          .with_bool_flag_default("newJsxConfig", true),
        ..PluginOptions::default()
      }),
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

    // Case 1: .js file with React configuration
    // Expected: is_jsx = true, pragmas set from React configuration

    // Test with v3 JSX configuration using JsTransformerConfig
    let config_with_jsx = JsTransformerConfig {
      jsx: Some(JsxOptions {
        pragma: Some("React.createElement".to_string()),
        pragma_fragment: Some("React.Fragment".to_string()),
        import_source: None,
        automatic_runtime: None,
      }),
      ..Default::default()
    };

    // Case 1: Test with JSX configuration
    transformer.config = config_with_jsx;
    let jsx_config = transformer.determine_jsx_configuration(&asset);
    let is_jsx = jsx_config.is_jsx;
    let jsx_pragma = jsx_config.jsx_pragma;
    let jsx_pragma_frag = jsx_config.jsx_pragma_frag;

    assert!(
      is_jsx,
      "JS file with React configuration should have JSX enabled"
    );
    assert_eq!(jsx_pragma, Some("React.createElement".to_string()));
    assert_eq!(jsx_pragma_frag, Some("React.Fragment".to_string()));

    // Case 3: .js file with React configuration
    // Expected: is_jsx = true, pragmas set from React configuration
    let config_with_react_2 = JsTransformerConfig {
      jsx: Some(JsxOptions {
        pragma: Some("React.createElement".to_string()),
        pragma_fragment: Some("React.Fragment".to_string()),
        import_source: None,
        automatic_runtime: None,
      }),
      ..Default::default()
    };

    transformer.config = config_with_react_2;
    let jsx_config = transformer.determine_jsx_configuration(&asset);
    let is_jsx_react_only = jsx_config.is_jsx;
    let jsx_pragma_react_only = jsx_config.jsx_pragma;
    let jsx_pragma_frag_react_only = jsx_config.jsx_pragma_frag;

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
    // Expected: is_jsx = true, pragmas set from React configuration if present
    let jsx_asset = Asset {
      file_type: FileType::Jsx,
      is_source: true,
      env: asset.env.clone(),
      ..Default::default()
    };

    let config_no_react_2 = JsTransformerConfig {
      jsx: None,
      ..Default::default()
    };

    transformer.config = config_no_react_2;
    let jsx_config = transformer.determine_jsx_configuration(&jsx_asset);
    let is_jsx_jsx_file = jsx_config.is_jsx;
    let jsx_pragma_jsx_file = jsx_config.jsx_pragma;
    let jsx_pragma_frag_jsx_file = jsx_config.jsx_pragma_frag;

    assert!(
      is_jsx_jsx_file,
      ".jsx files should always have JSX enabled (file extension check)"
    );
    // Note: .jsx files get JSX enabled by file extension, and get default pragmas
    assert_eq!(jsx_pragma_jsx_file, Some("React.createElement".to_string()));
    assert_eq!(jsx_pragma_frag_jsx_file, Some("React.Fragment".to_string()));

    // Case 5: .jsx file with React configuration should have pragmas set
    let config_with_react_3 = JsTransformerConfig {
      jsx: Some(JsxOptions {
        pragma: Some("React.createElement".to_string()),
        pragma_fragment: Some("React.Fragment".to_string()),
        import_source: None,
        automatic_runtime: None,
      }),
      ..Default::default()
    };

    transformer.config = config_with_react_3;
    let jsx_config = transformer.determine_jsx_configuration(&jsx_asset);
    let is_jsx_jsx_with_react = jsx_config.is_jsx;
    let jsx_pragma_jsx_with_react = jsx_config.jsx_pragma;
    let jsx_pragma_frag_jsx_with_react = jsx_config.jsx_pragma_frag;

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

    let config_with_react_4 = JsTransformerConfig {
      jsx: Some(JsxOptions {
        pragma: Some("React.createElement".to_string()),
        pragma_fragment: Some("React.Fragment".to_string()),
        import_source: None,
        automatic_runtime: None,
      }),
      ..Default::default()
    };

    transformer.config = config_with_react_4;
    let jsx_config = transformer.determine_jsx_configuration(&ts_asset);
    let is_jsx_ts = jsx_config.is_jsx;
    let jsx_pragma_ts = jsx_config.jsx_pragma;
    let jsx_pragma_frag_ts = jsx_config.jsx_pragma_frag;

    assert!(
      !is_jsx_ts,
      ".ts files should never have JSX enabled (explicitly disabled)"
    );
    assert_eq!(jsx_pragma_ts, None);
    assert_eq!(jsx_pragma_frag_ts, None);

    // Case 7: .tsx file should always have JSX enabled
    // Expected: is_jsx = true, pragmas set from React configuration if present
    let tsx_asset = Asset {
      file_type: FileType::Tsx,
      is_source: true,
      env: asset.env.clone(),
      ..Default::default()
    };

    let config_with_react_5 = JsTransformerConfig {
      jsx: Some(JsxOptions {
        pragma: Some("React.createElement".to_string()),
        pragma_fragment: Some("React.Fragment".to_string()),
        import_source: None,
        automatic_runtime: None,
      }),
      ..Default::default()
    };

    transformer.config = config_with_react_5;
    let jsx_config = transformer.determine_jsx_configuration(&tsx_asset);
    let is_jsx_tsx = jsx_config.is_jsx;
    let jsx_pragma_tsx = jsx_config.jsx_pragma;
    let jsx_pragma_frag_tsx = jsx_config.jsx_pragma_frag;

    assert!(
      is_jsx_tsx,
      ".tsx files should always have JSX enabled (file extension check)"
    );
    assert_eq!(jsx_pragma_tsx, Some("React.createElement".to_string()));
    assert_eq!(jsx_pragma_frag_tsx, Some("React.Fragment".to_string()));

    // Case 8: Test AutomaticReactRuntime::Glob functionality
    let test_asset = Asset {
      file_path: PathBuf::from("/src/components/Button.tsx"),
      file_type: FileType::Tsx,
      is_source: true,
      env: asset.env.clone(),
      ..Default::default()
    };

    let config_with_glob = JsTransformerConfig {
      jsx: Some(JsxOptions {
        pragma: Some("React.createElement".to_string()),
        pragma_fragment: Some("React.Fragment".to_string()),
        import_source: None,
        automatic_runtime: Some(AutomaticReactRuntime::Glob(AutomaticRuntimeGlobs {
          include: vec!["src/components/**/*.tsx".to_string()],
          exclude: None,
        })),
      }),
      ..Default::default()
    };

    transformer.config = config_with_glob;
    let jsx_config = transformer.determine_jsx_configuration(&test_asset);
    let automatic_jsx_runtime = jsx_config.automatic_jsx_runtime;

    assert_eq!(
      automatic_jsx_runtime, true,
      "Files matching glob pattern should have automatic JSX runtime enabled"
    );

    // Test with non-matching glob
    let config_with_non_matching_glob = JsTransformerConfig {
      jsx: Some(JsxOptions {
        pragma: Some("React.createElement".to_string()),
        pragma_fragment: Some("React.Fragment".to_string()),
        import_source: None,
        automatic_runtime: Some(AutomaticReactRuntime::Glob(AutomaticRuntimeGlobs {
          include: vec!["src/pages/**/*.tsx".to_string()],
          exclude: None,
        })),
      }),
      ..Default::default()
    };

    transformer.config = config_with_non_matching_glob;
    let jsx_config = transformer.determine_jsx_configuration(&test_asset);
    let automatic_jsx_runtime_no_match = jsx_config.automatic_jsx_runtime;

    assert!(
      !automatic_jsx_runtime_no_match,
      "Files not matching glob pattern should not have automatic JSX runtime enabled"
    );
  }

  // End-to-end tests for the new determine_jsx_configuration logic
  // These tests verify that the JSX configuration works correctly in the full transform pipeline

  mod jsx_configuration {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn js_file_with_tsconfig_and_react() -> anyhow::Result<()> {
      use atlaspack_test_fixtures::test_fixture;

      // Case 1: .js file with React dependency and tsconfig jsx: "react"
      // Expected: JSX should be enabled and transformed
      let project_root = PathBuf::from("/test");
      let file_system = test_fixture! {
        project_root.clone(),
        "package.json" => r#"{
          "dependencies": { "react": "^18.0.0" },
          "@atlaspack/transformer-js": {
            "jsx": {
              "pragma": "React.createElement",
              "pragmaFragment": "React.Fragment"
            }
          }
        }"#,
        "tsconfig.json" => r#"{
          "compilerOptions": {
            "jsx": "react",
            "target": "es2015"
          }
        }"#,
        "render.js" => r#"
          import React from 'react';

          const Component = () => {
            return <div>Hello World</div>;
          };

          export default Component;
        "#
      };

      let target_asset = create_asset_at_project_root(
        &project_root,
        "render.js",
        r#"
          import React from 'react';

          const Component = () => {
            return <div>Hello World</div>;
          };

          export default Component;
        "#,
      );

      let result = run_test(TestOptions {
        asset: target_asset,
        file_system: Some(file_system),
        project_root: Some(project_root),
        ..Default::default()
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

    // newJsxConfig CLEANUP NOTE: Remove this test after rollout.
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
        feature_flags: Some(FeatureFlags::with_bool_flag("newJsxConfig", false)),
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
    async fn js_file_with_react_only() -> anyhow::Result<()> {
      use atlaspack_test_fixtures::test_fixture;

      // Case 3: .js file with React dependency but NO tsconfig
      // Expected: JSX should be enabled and transformed
      let project_root = PathBuf::from("/test");
      let file_system = test_fixture! {
        project_root.clone(),
        "package.json" => r#"{
          "dependencies": { "react": "^18.0.0" },
          "@atlaspack/transformer-js": {
            "jsx": {
              "pragma": "React.createElement",
              "pragmaFragment": "React.Fragment"
            }
          }
        }"#,
        "render.js" => r#"
          import React from 'react';

          const Component = () => {
            return <div>Hello World</div>;
          };

          export default Component;
        "#
      };

      let target_asset = create_asset_at_project_root(
        &project_root,
        "render.js",
        r#"
          import React from 'react';

          const Component = () => {
            return <div>Hello World</div>;
          };

          export default Component;
        "#,
      );

      let result = run_test(TestOptions {
        asset: target_asset,
        file_system: Some(file_system),
        project_root: Some(project_root),
        ..Default::default()
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

    // newJsxConfig CLEANUP NOTE: Remove this test after rollout.
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
        feature_flags: Some(FeatureFlags::with_bool_flag("newJsxConfig", false)),
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
    env: BTreeMap<String, String>,
  }

  async fn run_test(options: TestOptions) -> anyhow::Result<TransformResult> {
    let asset = options.asset;
    let project_root = options.project_root.unwrap_or_default();
    let file_system = options
      .file_system
      .unwrap_or_else(|| default_fs(&project_root));

    // newJsxConfig CLEANUP NOTE: Remove 'flag enabled for tests by default' logic
    let plugin_options = PluginOptions {
      feature_flags: options
        .feature_flags
        .unwrap_or_default()
        .with_bool_flag_default("newJsxConfig", true),
      project_root: project_root.clone(),
      env: options.env,
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

    let mut result = transformer.transform(asset).await?;

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

  // Integration tests for environment variable filtering through the full transformer
  #[tokio::test(flavor = "multi_thread")]
  async fn test_transformer_with_inline_env_default() -> anyhow::Result<()> {
    use atlaspack_test_fixtures::test_fixture;

    let project_root = PathBuf::from("/test");
    let file_system = test_fixture! {
      project_root.clone(),
    };

    let source_asset = create_asset_at_project_root(
      &project_root,
      "index.js",
      r#"
        console.log(process.env.NODE_ENV);
        console.log(process.env.API_KEY);
      "#,
    );

    let mut env_vars = BTreeMap::new();
    env_vars.insert("NODE_ENV".to_string(), "production".to_string());
    env_vars.insert("API_KEY".to_string(), "secret".to_string());

    let result = run_test(TestOptions {
      asset: source_asset,
      file_system: Some(file_system.clone()),
      project_root: Some(project_root.clone()),
      env: env_vars,
      ..Default::default()
    })
    .await?;

    // With Enabled(false), only NODE_ENV should be inlined
    let code = result.asset.code.as_str()?;
    assert!(
      code.contains("\"production\""),
      "NODE_ENV should be inlined, got: {}",
      code
    );
    assert!(
      code.contains("\"secret\""),
      "API_KEY should be inlined, got: {}",
      code
    );

    let external_asset = create_asset_at_project_root(
      &project_root,
      "node_modules/library.js",
      r#"
        console.log(process.env.NODE_ENV);
        console.log(process.env.API_KEY);
      "#,
    );

    let mut env_vars = BTreeMap::new();
    env_vars.insert("NODE_ENV".to_string(), "production".to_string());
    env_vars.insert("API_KEY".to_string(), "secret".to_string());

    let result = run_test(TestOptions {
      asset: external_asset,
      file_system: Some(file_system),
      project_root: Some(project_root),
      env: env_vars,
      ..Default::default()
    })
    .await?;

    // With Enabled(false), only NODE_ENV should be inlined
    let code = result.asset.code.as_str()?;
    assert!(
      code.contains("\"production\""),
      "NODE_ENV should be inlined, got: {}",
      code
    );
    assert!(
      !code.contains("\"secret\""),
      "API_KEY should not be inlined, got: {}",
      code
    );

    Ok(())
  }

  // Integration tests for environment variable filtering through the full transformer
  #[tokio::test(flavor = "multi_thread")]
  async fn test_transformer_with_inline_env_disabled() -> anyhow::Result<()> {
    use atlaspack_test_fixtures::test_fixture;

    let project_root = PathBuf::from("/test");
    let file_system = test_fixture! {
      project_root.clone(),
      "package.json" => r#"{
        "@atlaspack/transformer-js": {
          "inlineEnvironment": false
        }
      }"#,
      "index.js" => r#"
        console.log(process.env.NODE_ENV);
        console.log(process.env.API_KEY);
      "#
    };

    let target_asset = create_asset_at_project_root(
      &project_root,
      "index.js",
      r#"
        console.log(process.env.NODE_ENV);
        console.log(process.env.API_KEY);
      "#,
    );

    let mut env_vars = BTreeMap::new();
    env_vars.insert("NODE_ENV".to_string(), "production".to_string());
    env_vars.insert("API_KEY".to_string(), "secret".to_string());

    let result = run_test(TestOptions {
      asset: target_asset,
      file_system: Some(file_system),
      project_root: Some(project_root),
      env: env_vars,
      ..Default::default()
    })
    .await?;

    // With Enabled(false), only NODE_ENV should be inlined
    let code = result.asset.code.as_str()?;
    assert!(
      code.contains("\"production\""),
      "NODE_ENV should be inlined, got: {}",
      code
    );
    assert!(
      !code.contains("\"secret\""),
      "API_KEY should NOT be inlined, got: {}",
      code
    );

    Ok(())
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_transformer_with_inline_env_enabled() -> anyhow::Result<()> {
    use atlaspack_test_fixtures::test_fixture;

    let project_root = PathBuf::from("/test");
    let file_system = test_fixture! {
      project_root.clone(),
      "package.json" => r#"{
        "@atlaspack/transformer-js": {
          "inlineEnvironment": true
        }
      }"#,
      "index.js" => r#"
        console.log(process.env.NODE_ENV);
        console.log(process.env.CUSTOM_VAR);
      "#
    };

    let target_asset = create_asset_at_project_root(
      &project_root,
      "index.js",
      r#"
        console.log(process.env.NODE_ENV);
        console.log(process.env.CUSTOM_VAR);
      "#,
    );

    let mut env_vars = BTreeMap::new();
    env_vars.insert("NODE_ENV".to_string(), "production".to_string());
    env_vars.insert("CUSTOM_VAR".to_string(), "custom-var".to_string());

    let result = run_test(TestOptions {
      asset: target_asset,
      file_system: Some(file_system),
      project_root: Some(project_root),
      env: env_vars,
      ..Default::default()
    })
    .await?;

    // With Enabled(true), all environment variables should be inlined
    let code = result.asset.code.as_str()?;
    assert!(
      code.contains("\"production\""),
      "NODE_ENV should be inlined, got: {}",
      code
    );
    assert!(
      code.contains("\"custom-var\""),
      "CUSTOM_VAR should be inlined, got: {}",
      code
    );

    Ok(())
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn test_transformer_with_inline_env_glob_patterns() -> anyhow::Result<()> {
    use atlaspack_test_fixtures::test_fixture;

    let project_root = PathBuf::from("/test");
    let file_system = test_fixture! {
      project_root.clone(),
      "package.json" => r#"{
        "@atlaspack/transformer-js": {
          "inlineEnvironment": ["REACT_APP_*", "NODE_ENV"]
        }
      }"#,
      "index.js" => r#"
        console.log(process.env.REACT_APP_VERSION);
        console.log(process.env.API_SECRET);
        console.log(process.env.NODE_ENV);
      "#
    };

    let target_asset = create_asset_at_project_root(
      &project_root,
      "index.js",
      r#"
        console.log(process.env.REACT_APP_VERSION);
        console.log(process.env.API_SECRET);
        console.log(process.env.NODE_ENV);
      "#,
    );

    let mut env_vars = BTreeMap::new();
    env_vars.insert(
      "REACT_APP_VERSION".to_string(),
      "react-app-version".to_string(),
    );
    env_vars.insert("API_SECRET".to_string(), "api-secret".to_string());
    env_vars.insert("NODE_ENV".to_string(), "production".to_string());

    let result = run_test(TestOptions {
      asset: target_asset,
      file_system: Some(file_system),
      project_root: Some(project_root),
      env: env_vars,
      ..Default::default()
    })
    .await?;

    // With glob patterns, only REACT_APP_* and NODE_ENV should be inlined
    let code = result.asset.code.as_str()?;
    assert!(
      code.contains("\"react-app-version\""),
      "REACT_APP_VERSION should be inlined, got: {}",
      code
    );
    assert!(
      code.contains("\"production\""),
      "NODE_ENV should be inlined, got: {}",
      code
    );
    assert!(
      !code.contains("\"api-secret\""),
      "API_SECRET should NOT be inlined, got: {}",
      code
    );

    Ok(())
  }
}
