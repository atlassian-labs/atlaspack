use std::collections::BTreeMap;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Error};
use async_trait::async_trait;
use atlaspack_core::plugin::{PluginContext, TransformerPlugin};
use atlaspack_core::plugin::{TransformContext, TransformResult};
use atlaspack_core::types::engines::{Engines, EnginesBrowsers};
use atlaspack_core::types::{
  Asset, AssetWithDependencies, Code, Dependency, Diagnostic, EnvironmentContext, ErrorKind,
  ExportsCondition, FileType, Priority, SourceMap, SpecifierType, Symbol,
};
use lightningcss::css_modules::CssModuleExport;
use lightningcss::dependencies::DependencyOptions;
use lightningcss::printer::PrinterOptions;
use lightningcss::stylesheet::{ParserFlags, ParserOptions, StyleSheet};
use lightningcss::targets::{Browsers, Targets};
use parcel_sourcemap::SourceMap as ParcelSourceMap;
use serde::Deserialize;
use serde_json::json;

use crate::css_transformer_config::{CssModulesConfig, CssModulesFullConfig, CssTransformerConfig};

#[derive(Debug)]
pub struct AtlaspackCssTransformerPlugin {
  project_root: PathBuf,
  css_modules_config: CssModulesFullConfig,
}

#[derive(Deserialize)]
struct PackageJson {
  #[serde(rename = "@atlaspack/transformer-css")]
  config: Option<CssTransformerConfig>,
}

impl AtlaspackCssTransformerPlugin {
  pub fn new(ctx: &PluginContext) -> Result<Self, Error> {
    let config = ctx.config.load_package_json::<PackageJson>().map_or_else(
      |err| {
        let diagnostic = err.downcast_ref::<Diagnostic>();

        if diagnostic.is_some_and(|d| d.kind != ErrorKind::NotFound) {
          return Err(err);
        }

        Ok(CssTransformerConfig::default())
      },
      |config| Ok(config.contents.config.unwrap_or_default()),
    )?;

    let css_modules_config = config
      .css_modules
      .map(|css_modules_config| match css_modules_config {
        CssModulesConfig::GlobalOnly(global) => CssModulesFullConfig {
          global: Some(global),
          ..CssModulesFullConfig::default()
        },
        CssModulesConfig::Full(config) => config,
      })
      .unwrap_or_default();

    Ok(AtlaspackCssTransformerPlugin {
      project_root: ctx.options.project_root.clone(),
      css_modules_config,
    })
  }

  fn is_css_module(&self, asset: &Asset) -> bool {
    let is_style_tag = asset
      .meta
      .get("type")
      .is_some_and(|meta_type| *meta_type == json!("tag"));

    // If this is a style tag it's not a CSS module
    if is_style_tag {
      return false;
    }

    let matches_css_module_file_pattern = asset
      .file_path
      .file_name()
      .is_some_and(|name| name.to_string_lossy().ends_with(".module.css"));

    // If it matches the *.module.css pattern, it is a CSS module
    if matches_css_module_file_pattern {
      return true;
    }

    // TODO: Implement include and exclude globs

    // Otherwise if the asset is a source asset and global CSS modules are
    // enabled
    asset.is_source && self.css_modules_config.global.unwrap_or_default()
  }
}

#[async_trait]
impl TransformerPlugin for AtlaspackCssTransformerPlugin {
  async fn transform(
    &self,
    _context: TransformContext,
    asset: Asset,
  ) -> Result<TransformResult, Error> {
    let css_modules = if self.is_css_module(&asset) {
      Some(lightningcss::css_modules::Config {
        dashed_idents: asset.is_source && self.css_modules_config.dashed_idents.unwrap_or_default(),
        ..Default::default()
      })
    } else {
      None
    };

    let stylesheet = StyleSheet::parse(
      asset.code.as_str()?,
      ParserOptions {
        filename: asset
          .file_path
          .clone()
          .into_os_string()
          .into_string()
          .map_err(|_e| anyhow!("Couldn't convert file path to String"))?,
        css_modules,
        source_index: Default::default(),
        error_recovery: false,
        warnings: None,
        flags: ParserFlags::empty(),
      },
    )
    .map_err(|_err| {
      // TODO: Proper error handling
      anyhow!("Failed to parse CSS {}", asset.file_path.display())
    })?;

    let mut asset = asset.clone();

    // Normalize the asset's environment so that properties that only affect JS don't cause CSS to be duplicated.
    // For example, with ESModule and CommonJS targets, only a single shared CSS bundle should be produced.
    asset.env = Arc::new(atlaspack_core::types::Environment {
      context: EnvironmentContext::Browser,
      engines: Engines {
        browsers: asset.env.engines.browsers.clone(),
        ..Default::default()
      },
      ..asset.env.deref().clone()
    });

    let preserve_imports = asset
      .meta
      .get("hasDependencies")
      .map_or_else(|| true, |value| value != false);

    let browsers = asset.env.engines.browsers.clone().map_or_else(
      || Ok(None),
      |browsers| match browsers {
        EnginesBrowsers::String(s) => Browsers::from_browserslist(vec![s]),
        EnginesBrowsers::List(l) => Browsers::from_browserslist(l),
      },
    )?;

    let mut lightning_source_map: Option<ParcelSourceMap> = if asset.env.source_map.is_some() {
      let mut sm = ParcelSourceMap::new(&self.project_root.to_string_lossy());
      sm.add_source(&asset.file_path.to_string_lossy());
      sm.set_source_content(0, asset.code.as_str()?)?;
      Some(sm)
    } else {
      None
    };

    let css = stylesheet.to_css(PrinterOptions {
      minify: false,
      source_map: lightning_source_map.as_mut(),
      project_root: self.project_root.to_str(),
      targets: Targets {
        browsers,
        include: Default::default(),
        exclude: Default::default(),
      },
      analyze_dependencies: Some(DependencyOptions {
        remove_imports: !preserve_imports,
      }),
      pseudo_classes: None,
    })?;

    let mut dependencies: Vec<Dependency> = css
      .dependencies
      .as_ref()
      .map(|dependencies| {
        dependencies
          .iter()
          .filter_map(|dependency| match dependency {
            lightningcss::dependencies::Dependency::Import(import_dependency) => {
              if css.exports.is_some() {
                // When exports from a CSS module are available, we handle the
                // dependencies separately
                return None;
              }

              let mut dependency = Dependency {
                env: asset.env.clone(),
                package_conditions: ExportsCondition::STYLE,
                priority: Priority::Sync,
                source_asset_id: Some(asset.id.clone()),
                source_path: Some(asset.file_path.clone()),
                specifier: import_dependency.url.clone(),
                specifier_type: SpecifierType::Url,
                source_asset_type: Some(FileType::Css),
                ..Dependency::default()
              };

              if let Some(media) = &import_dependency.media {
                dependency.meta.insert("media".into(), media.clone().into());
              }

              // For the glob resolver to distinguish between `@import` and other URL dependencies.
              dependency.meta.insert("isCSSImport".into(), true.into());

              dependency.set_placeholder(import_dependency.placeholder.clone());

              Some(dependency)
            }
            lightningcss::dependencies::Dependency::Url(url_dependency) => {
              let mut dependency = Dependency {
                env: asset.env.clone(),
                priority: Priority::Sync,
                source_asset_id: Some(asset.id.clone()),
                source_asset_type: Some(FileType::Css),
                source_path: Some(asset.file_path.clone()),
                specifier: url_dependency.url.clone(),
                specifier_type: SpecifierType::Url,
                ..Dependency::default()
              };

              dependency.set_placeholder(url_dependency.placeholder.clone());

              Some(dependency)
            }
          })
          .collect()
      })
      .unwrap_or_default();

    let mut css_code = Vec::new();
    let mut discovered_assets = Vec::new();
    let mut asset_symbols: Vec<Symbol> = Vec::new();

    if let Some(exports) = css.exports {
      let mut export_code = String::new();

      // Set the unique key of the root asset so we can use it to assign some generated
      // dependencies to it
      let css_unique_key = asset.id.clone();
      asset.unique_key = Some(css_unique_key.clone());

      asset_symbols.push(Symbol {
        exported: "default".into(),
        local: "default".into(),
        is_weak: false,
        is_esm_export: true,
        self_referenced: false,
        loc: None,
      });

      // It's possible that the exports can be ordered differently between builds.
      // Sorting by key is safe as the order is irrelevant but needs to be deterministic.
      let sorted_exports: BTreeMap<String, CssModuleExport> = exports.into_iter().collect();
      for (key, export) in sorted_exports.iter() {
        if !export.composes.is_empty() {
          return Err(anyhow!(
            "CSS module 'composes' not currently supported in Atlaspack V3"
          ));
        }

        asset_symbols.push(Symbol {
          exported: key.clone(),
          local: export.name.clone(),
          is_weak: false,
          is_esm_export: true,
          self_referenced: false,
          loc: None,
        });
        export_code
          .push_str(format!("module.exports[\"{}\"] = `{}`;\n", key, export.name).as_str());

        // If the export is referenced internally (e.g. used @keyframes), add a self-reference
        // to the JS so the symbol is retained during tree-shaking.
        if export.is_referenced {
          export_code.push_str(format!("module.exports[\"{key}\"];\n").as_str());

          let symbols = vec![Symbol {
            exported: key.clone(),
            local: export.name.clone(),
            is_weak: false,
            is_esm_export: false,
            self_referenced: true,
            loc: None,
          }];

          dependencies.push(Dependency {
            // Point this at the root asset
            specifier: css_unique_key.clone(),
            specifier_type: SpecifierType::Esm,
            symbols: Some(symbols),
            env: asset.env.clone(),
            source_asset_id: Some(asset.id.clone()),
            source_path: Some(asset.file_path.clone()),
            source_asset_type: Some(FileType::Css),
            ..Dependency::default()
          });
        }
      }

      let mut import_code = String::new();

      if let Some(dependencies) = &css.dependencies {
        for (index, dependency) in dependencies.iter().enumerate() {
          if let lightningcss::dependencies::Dependency::Import(import) = dependency {
            let local = format!("dep_${index}");

            let import_statement = format!("import * as {} from \"{}\";\n", local, import.url);
            import_code.push_str(&import_statement);

            let export_statement = format!(
              r"
              for (let key in {local}) {{
                if (key in module.exports)
                  module.exports[key] += ' ' + {local}[key];
                else
                  module.exports[key] = {local}[key];
              }}
            "
            );
            export_code.push_str(&export_statement);

            asset_symbols.push(Symbol {
              exported: "*".into(),
              local: "*".into(),
              is_weak: false,
              is_esm_export: true,
              self_referenced: false,
              loc: None,
            })
          }
        }
      }

      if let Some(references) = css.references {
        for (local, reference) in references.iter() {
          if let lightningcss::css_modules::CssModuleReference::Dependency { name, specifier } =
            reference
          {
            let symbols = vec![Symbol {
              local: local.clone(),
              exported: name.clone(),
              is_weak: false,
              loc: None,
              self_referenced: false,
              is_esm_export: false,
            }];

            dependencies.push(Dependency {
              specifier: specifier.clone(),
              specifier_type: SpecifierType::Esm,
              package_conditions: ExportsCondition::STYLE,
              symbols: Some(symbols),
              env: asset.env.clone(),
              source_asset_id: Some(asset.id.clone()),
              source_path: Some(asset.file_path.clone()),
              source_asset_type: Some(FileType::Css),
              ..Dependency::default()
            });

            asset.meta.insert("hasReferences".into(), true.into());
            css_code.push(format!("@import '{}';", specifier));
          }
        }
      }

      let discovered_asset = Asset::new_discovered(
        &asset,
        None,
        FileType::Js,
        format!("{import_code}{export_code}"),
      )?;

      discovered_assets.push(AssetWithDependencies {
        asset: discovered_asset,
        dependencies: Vec::new(),
      });
    }

    if !asset_symbols.is_empty() {
      if let Some(symbols) = asset.symbols.as_mut() {
        symbols.extend(asset_symbols);
      } else {
        asset.symbols = Some(asset_symbols);
      }
    }

    // Add the generated css imports to the css output
    css_code.push(css.code);
    asset.code = Arc::new(Code::from(css_code.join("\n")));

    if let Some(source_map) = lightning_source_map.clone() {
      let mut source_map = SourceMap::from(source_map);

      if let Some(original_map) = asset.map {
        source_map.extends(&mut original_map.clone())?;
      }

      asset.map = Some(source_map);
    }

    Ok(TransformResult {
      asset,
      dependencies,
      discovered_assets,
      ..Default::default()
    })
  }
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;
  use std::{path::PathBuf, sync::Arc};

  use atlaspack_core::{
    config_loader::ConfigLoader,
    plugin::{PluginLogger, PluginOptions},
    types::JSONObject,
  };
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;

  use super::*;

  async fn run_plugin(asset: &Asset) -> anyhow::Result<TransformResult> {
    let file_system = Arc::new(InMemoryFileSystem::default());
    let plugin = AtlaspackCssTransformerPlugin::new(&PluginContext {
      config: Arc::new(ConfigLoader {
        fs: file_system.clone(),
        project_root: PathBuf::default(),
        search_path: PathBuf::default(),
      }),
      file_system,
      logger: PluginLogger::default(),
      options: Arc::new(PluginOptions::default()),
    })?;
    let context = TransformContext::default();

    plugin.transform(context, asset.clone()).await
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn supports_css_imports() {
    let asset = Asset {
      id: "my-asset".into(),
      file_path: "styles.css".into(),
      code: Arc::new(Code::from("@import './stuff.css';")),
      ..Default::default()
    };
    let result = run_plugin(&asset).await;

    assert_eq!(
      result.unwrap().dependencies,
      vec![Dependency {
        specifier: "./stuff.css".into(),
        source_asset_id: Some("my-asset".into()),
        source_path: Some("styles.css".into()),
        source_asset_type: Some(FileType::Css),
        specifier_type: SpecifierType::Url,
        package_conditions: ExportsCondition::STYLE,
        meta: JSONObject::from_iter([
          ("isCSSImport".into(), true.into()),
          ("placeholder".into(), "OFe21q".into())
        ]),
        ..Dependency::default()
      }]
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn supports_css_modules() {
    let asset = Asset {
      id: "css-module".into(),
      file_path: "styles.module.css".into(),
      is_source: true,
      code: Arc::new(Code::from(".root {display: 'block'}")),
      ..Default::default()
    };

    let result = run_plugin(&asset).await.unwrap();

    assert_eq!(
      result.asset,
      Asset {
        code: Arc::new(".EcQGha_root {\n  display: \"block\";\n}\n".into()),
        unique_key: Some("css-module".into()),
        symbols: Some(vec![
          Symbol {
            local: "default".into(),
            exported: "default".into(),
            loc: None,
            is_weak: false,
            is_esm_export: true,
            self_referenced: false,
          },
          Symbol {
            local: "EcQGha_root".into(),
            exported: "root".into(),
            loc: None,
            is_weak: false,
            is_esm_export: true,
            self_referenced: false,
          },
        ]),
        ..asset
      }
    );
    assert_eq!(result.discovered_assets.len(), 1);
    assert_eq!(
      result.discovered_assets[0],
      AssetWithDependencies {
        asset: Asset {
          id: "88540641b9eed86d".into(),
          code: Arc::new("module.exports[\"root\"] = `EcQGha_root`;\n".into()),
          file_path: "styles.module.css".into(),
          is_source: true,
          ..Default::default()
        },
        dependencies: Vec::new()
      }
    );
  }
}
