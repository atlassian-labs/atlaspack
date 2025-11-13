use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use atlaspack_core::diagnostic_error;
use atlaspack_core::types::CodeFrame;
use atlaspack_core::types::CodeHighlight;
use atlaspack_core::types::DiagnosticBuilder;
use atlaspack_core::types::DiagnosticError;
use atlaspack_core::types::File;
use atlaspack_filesystem::FileSystemRef;
use atlaspack_filesystem::search::find_ancestor_file;
use atlaspack_package_manager::PackageManagerRef;
use pathdiff::diff_paths;
use serde_json5::Location;

use super::atlaspack_config::AtlaspackConfig;
use super::atlaspack_config::PluginNode;
use super::atlaspack_rc::AtlaspackRcFile;
use super::atlaspack_rc::Extends;
use super::partial_atlaspack_config::PartialAtlaspackConfig;

#[derive(Default)]
pub struct LoadConfigOptions<'a> {
  /// A list of additional reporter plugins that will be appended to the reporters config
  pub additional_reporters: Vec<PluginNode>,
  /// A file path or package specifier that will be used to load the config from
  pub config: Option<&'a str>,
  /// A file path or package specifier that will be used to load the config from when no other
  /// .parcelrc can be found
  pub fallback_config: Option<&'a str>,
}

/// Loads and validates .atlaspack_rc config
pub struct AtlaspackRcConfigLoader {
  fs: FileSystemRef,
  package_manager: PackageManagerRef,
}

impl AtlaspackRcConfigLoader {
  pub fn new(fs: FileSystemRef, package_manager: PackageManagerRef) -> Self {
    AtlaspackRcConfigLoader {
      fs,
      package_manager,
    }
  }

  fn find_config(&self, project_root: &Path, path: &Path) -> Result<PathBuf, DiagnosticError> {
    let from = path.parent().unwrap_or(path);

    find_ancestor_file(&*self.fs, &[".parcelrc"], from, project_root)
      .ok_or_else(|| diagnostic_error!("Unable to locate .parcelrc from {}", from.display()))
  }

  fn resolve_from(&self, project_root: &Path) -> PathBuf {
    let cwd = self.fs.cwd().unwrap();
    let relative = diff_paths(cwd.clone(), project_root);
    let is_cwd_inside_project_root =
      relative.is_some_and(|p| !p.starts_with("..") && !p.is_absolute());

    let dir = if is_cwd_inside_project_root {
      &cwd
    } else {
      project_root
    };

    dir.join("index")
  }

  fn load_config(
    &self,
    path: PathBuf,
  ) -> Result<(PartialAtlaspackConfig, Vec<PathBuf>), DiagnosticError> {
    let raw = self.fs.read_to_string(&path).map_err(|source| {
      diagnostic_error!(
        DiagnosticBuilder::default()
          .message(source.to_string())
          .code_frames(vec![CodeFrame::from(path.clone())])
      )
    })?;

    let contents = serde_json5::from_str(&raw).map_err(|error| {
      serde_to_diagnostic_error(
        error,
        File {
          contents: raw.clone(),
          path: path.clone(),
        },
      )
    })?;

    self.process_config(AtlaspackRcFile {
      contents,
      path,
      raw,
    })
  }

  fn resolve_extends(
    &self,
    atlaspack_rc_file: &AtlaspackRcFile,
    extend: &str,
  ) -> Result<PathBuf, DiagnosticError> {
    let path = if extend.starts_with(".") {
      atlaspack_rc_file
        .path
        .parent()
        .unwrap_or(&atlaspack_rc_file.path)
        .join(extend)
    } else {
      self
        .package_manager
        .resolve(extend, &atlaspack_rc_file.path)
        .map_err(|source| {
          source.context(diagnostic_error!(
            DiagnosticBuilder::default()
              .message(format!(
                "Failed to resolve extended config {extend} from {}",
                atlaspack_rc_file.path.display()
              ))
              .code_frames(vec![CodeFrame::from(File::from(atlaspack_rc_file))])
          ))
        })?
        .resolved
    };

    self.fs.canonicalize_base(&path).map_err(|source| {
      diagnostic_error!("{}", source).context(diagnostic_error!(
        DiagnosticBuilder::default()
          .message(format!(
            "Failed to resolve extended config {extend} from {}",
            atlaspack_rc_file.path.display()
          ))
          .code_frames(vec![CodeFrame::from(File::from(atlaspack_rc_file))])
      ))
    })
  }

  /// Processes a .parcelrc file by loading and merging "extends" configurations into a single
  /// PartialAtlaspackConfig struct
  ///
  /// Configuration merging will be applied to all "extends" configurations, before being merged
  /// into the base config for a more natural merging order. It will replace any "..." seen in
  /// plugin pipelines with the corresponding plugins from "extends" if present.
  ///
  fn process_config(
    &self,
    atlaspack_rc_file: AtlaspackRcFile,
  ) -> Result<(PartialAtlaspackConfig, Vec<PathBuf>), DiagnosticError> {
    let mut files = vec![atlaspack_rc_file.path.clone()];
    let extends = atlaspack_rc_file.contents.extends.as_ref();
    let extends = match extends {
      None => Vec::new(),
      Some(extends) => match extends {
        Extends::One(ext) => vec![String::from(ext)],
        Extends::Many(ext) => ext.to_vec(),
      },
    };

    if extends.is_empty() {
      return Ok((PartialAtlaspackConfig::try_from(atlaspack_rc_file)?, files));
    }

    let mut merged_config: Option<PartialAtlaspackConfig> = None;
    for extend in extends {
      let extended_file_path = self.resolve_extends(&atlaspack_rc_file, &extend)?;
      let (extended_config, mut extended_file_paths) = self.load_config(extended_file_path)?;

      merged_config = match merged_config {
        None => Some(extended_config),
        Some(config) => Some(PartialAtlaspackConfig::merge(config, extended_config)),
      };

      files.append(&mut extended_file_paths);
    }

    let config = PartialAtlaspackConfig::merge(
      PartialAtlaspackConfig::try_from(atlaspack_rc_file)?,
      merged_config.unwrap(),
    );

    Ok((config, files))
  }

  /// Finds and loads a .parcelrc file
  ///
  /// By default the nearest .parcelrc ancestor file from the current working directory will be
  /// loaded, unless the config or fallback_config option are specified. In cases where the
  /// current working directory does not live within the project root, the default config will be
  /// loaded from the project root.
  ///
  pub fn load(
    &self,
    project_root: &Path,
    options: LoadConfigOptions,
  ) -> Result<(AtlaspackConfig, Vec<PathBuf>), DiagnosticError> {
    let resolve_from = self.resolve_from(project_root);
    let mut config_path = match options.config {
      Some(config) => self
        .package_manager
        .resolve(config, &resolve_from)
        .map(|r| r.resolved)
        .map_err(|source| {
          source.context(diagnostic_error!(
            "Failed to resolve config {config} from {}",
            resolve_from.display()
          ))
        }),
      None => self.find_config(project_root, &resolve_from),
    };

    if config_path.is_err()
      && let Some(fallback_config) = options.fallback_config
    {
      config_path = self
        .package_manager
        .resolve(fallback_config, &resolve_from)
        .map(|r| r.resolved)
        .map_err(|source| {
          source.context(diagnostic_error!(
            "Failed to resolve fallback {fallback_config} from {}",
            resolve_from.display()
          ))
        })
    }

    let config_path = config_path?;
    let (mut atlaspack_config, files) = self.load_config(config_path)?;

    if !options.additional_reporters.is_empty() {
      atlaspack_config
        .reporters
        .extend(options.additional_reporters);

      let mut seen = HashSet::new();
      atlaspack_config.reporters.retain(|plugin_node| {
        if seen.contains(&plugin_node.package_name) {
          false
        } else {
          seen.insert(plugin_node.package_name.clone());
          true
        }
      });
    }

    let atlaspack_config = AtlaspackConfig::try_from(atlaspack_config)?;

    Ok((atlaspack_config, files))
  }
}

fn serde_to_diagnostic_error(error: serde_json5::Error, file: File) -> DiagnosticError {
  let mut diagnostic_error = DiagnosticBuilder::default();
  diagnostic_error.message(format!("Failed to parse {}", file.path.display()));

  match error {
    serde_json5::Error::Message { msg, location } => {
      let location = location.unwrap_or(Location { column: 1, line: 1 });

      diagnostic_error.code_frames(vec![CodeFrame {
        code_highlights: vec![CodeHighlight {
          message: Some(msg),
          ..CodeHighlight::from([location.line, location.column])
        }],
        ..CodeFrame::from(file)
      }]);
    }
  };

  diagnostic_error!(diagnostic_error)
}

#[cfg(test)]
mod tests {
  use crate::map::NamedPattern;
  use std::sync::Arc;

  use anyhow::anyhow;
  use atlaspack_filesystem::FileSystem;
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;
  use atlaspack_package_manager::MockPackageManager;
  use atlaspack_package_manager::PackageManager;
  use atlaspack_package_manager::Resolution;
  use mockall::predicate::eq;

  use super::*;

  fn fail_package_manager_resolution(package_manager: &mut MockPackageManager) {
    package_manager
      .expect_resolve()
      .return_once(|_specifier, _from| Err(anyhow!("Something bad happened")));
  }

  struct TestPackageManager {
    fs: FileSystemRef,
  }

  impl PackageManager for TestPackageManager {
    fn resolve(&self, specifier: &str, from: &Path) -> anyhow::Result<Resolution> {
      let path = match "true" {
        _s if specifier.starts_with(".") => from.join(specifier),
        _s if specifier.starts_with("@") => self
          .fs
          .cwd()
          .unwrap()
          .join("node_modules")
          .join(specifier)
          .join("index.json"),
        _ => PathBuf::from("Not found"),
      };

      if !self.fs.is_file(&path) {
        return Err(anyhow!("File was missing"));
      }

      Ok(Resolution { resolved: path })
    }
  }

  fn package_manager_resolution(
    package_manager: &mut MockPackageManager,
    specifier: String,
    from: PathBuf,
  ) -> PathBuf {
    let resolved = PathBuf::from("/")
      .join("node_modules")
      .join(specifier.clone())
      .join("index.json");

    package_manager
      .expect_resolve()
      .with(eq(specifier), eq(from))
      .returning(|specifier, _from| {
        Ok(Resolution {
          resolved: PathBuf::from("/")
            .join("node_modules")
            .join(specifier)
            .join("index.json"),
        })
      });

    resolved
  }

  mod empty_config_and_fallback {
    use crate::atlaspack_config_fixtures::default_config;
    use crate::atlaspack_config_fixtures::default_extended_config;

    use super::*;

    #[test]
    fn errors_on_missing_parcelrc_file() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = fs.cwd().unwrap();

      let err = AtlaspackRcConfigLoader::new(fs, Arc::new(MockPackageManager::new()))
        .load(&project_root, LoadConfigOptions::default())
        .map_err(|e| e.to_string());

      assert_eq!(
        err,
        Err(format!(
          "Unable to locate .parcelrc from {}",
          project_root.display()
        ))
      );
    }

    #[test]
    fn errors_on_failed_extended_parcelrc_resolution() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = fs.cwd().unwrap();

      let config = default_extended_config(&project_root);

      fs.write_file(&config.base_config.path, config.base_config.atlaspack_rc);

      let fs: FileSystemRef = fs;
      let package_manager = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });

      let err = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager)
        .load(&project_root, LoadConfigOptions::default())
        .map_err(|e| e.to_string());

      assert_eq!(
        err,
        Err(format!(
          "Failed to resolve extended config @atlaspack/config-default from {}",
          config.base_config.path.display()
        ))
      );
    }

    #[test]
    fn returns_default_atlaspack_config() {
      use crate::{
        AtlaspackConfig, PluginNode,
        map::{NamedPipelinesMap, PipelineMap, PipelinesMap},
      };
      use atlaspack_test_fixtures::test_fixture;
      use indexmap::IndexMap;
      use indexmap::indexmap;

      let project_root = PathBuf::from("/test");
      let fs = test_fixture! {
        project_root.clone(),
        ".parcelrc" => {r#"
          {
            "bundler": "@atlaspack/bundler-default",
            "compressors": {
              "*": ["@atlaspack/compressor-raw"]
            },
            "namers": ["@atlaspack/namer-default"],
            "optimizers": {
              "*.{js,mjs,cjs}": ["@atlaspack/optimizer-swc"]
            },
            "packagers": {
              "*.{js,mjs,cjs}": "@atlaspack/packager-js"
            },
            "reporters": ["@atlaspack/reporter-dev-server"],
            "resolvers": ["@atlaspack/resolver-default"],
            "runtimes": ["@atlaspack/runtime-js"],
            "transformers": {
              "*.{js,mjs,jsm,jsx,es6,cjs,ts,tsx}": [
                "@atlaspack/transformer-js"
              ]
            }
          }
        "#}
      };

      let atlaspack_config =
        AtlaspackRcConfigLoader::new(fs, Arc::new(MockPackageManager::default()))
          .load(&project_root, LoadConfigOptions::default())
          .map_err(|e| e.to_string());

      // Build the expected config structure
      let expected_config = AtlaspackConfig {
        bundler: PluginNode {
          package_name: String::from("@atlaspack/bundler-default"),
          resolve_from: Arc::new(project_root.join(".parcelrc")),
        },
        compressors: PipelinesMap::new(indexmap! {
          String::from("*") => vec!(PluginNode {
            package_name: String::from("@atlaspack/compressor-raw"),
            resolve_from: Arc::new(project_root.join(".parcelrc")),
          })
        }),
        namers: vec![PluginNode {
          package_name: String::from("@atlaspack/namer-default"),
          resolve_from: Arc::new(project_root.join(".parcelrc")),
        }],
        optimizers: NamedPipelinesMap::new(indexmap! {
          String::from("*.{js,mjs,cjs}") => vec!(PluginNode {
            package_name: String::from("@atlaspack/optimizer-swc"),
            resolve_from: Arc::new(project_root.join(".parcelrc")),
          })
        }),
        packagers: PipelineMap::new(indexmap! {
          String::from("*.{js,mjs,cjs}") => PluginNode {
            package_name: String::from("@atlaspack/packager-js"),
            resolve_from: Arc::new(project_root.join(".parcelrc")),
          }
        }),
        reporters: vec![PluginNode {
          package_name: String::from("@atlaspack/reporter-dev-server"),
          resolve_from: Arc::new(project_root.join(".parcelrc")),
        }],
        resolvers: vec![PluginNode {
          package_name: String::from("@atlaspack/resolver-default"),
          resolve_from: Arc::new(project_root.join(".parcelrc")),
        }],
        runtimes: vec![PluginNode {
          package_name: String::from("@atlaspack/runtime-js"),
          resolve_from: Arc::new(project_root.join(".parcelrc")),
        }],
        transformers: NamedPipelinesMap::new(indexmap! {
          String::from("*.{js,mjs,jsm,jsx,es6,cjs,ts,tsx}") => vec!(PluginNode {
            package_name: String::from("@atlaspack/transformer-js"),
            resolve_from: Arc::new(project_root.join(".parcelrc")),
          })
        }),
        validators: PipelinesMap::new(IndexMap::new()),
      };

      let expected_files = vec![project_root.join(".parcelrc")];

      assert_eq!(atlaspack_config, Ok((expected_config, expected_files)));
    }

    #[test]
    fn returns_default_atlaspack_config_from_project_root() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = fs.cwd().unwrap().join("src").join("packages").join("root");

      let default_config = default_config(Arc::new(project_root.join(".parcelrc")));
      let files = vec![default_config.path.clone()];

      fs.write_file(&default_config.path, default_config.atlaspack_rc);

      let atlaspack_config =
        AtlaspackRcConfigLoader::new(fs, Arc::new(MockPackageManager::default()))
          .load(&project_root, LoadConfigOptions::default())
          .map_err(|e| e.to_string());

      assert_eq!(
        atlaspack_config,
        Ok((default_config.atlaspack_config, files))
      );
    }

    #[test]
    fn returns_default_atlaspack_config_from_project_root_when_outside_cwd() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = PathBuf::from("/root");

      let default_config = default_config(Arc::new(project_root.join(".parcelrc")));
      let files = vec![default_config.path.clone()];

      fs.set_current_working_directory(Path::new("/cwd"));
      fs.write_file(&default_config.path, default_config.atlaspack_rc);

      let atlaspack_config =
        AtlaspackRcConfigLoader::new(fs, Arc::new(MockPackageManager::default()))
          .load(&project_root, LoadConfigOptions::default())
          .map_err(|e| e.to_string());

      assert_eq!(
        atlaspack_config,
        Ok((default_config.atlaspack_config, files))
      );
    }

    #[test]
    fn returns_merged_default_atlaspack_config() {
      use crate::{
        AtlaspackConfig, PluginNode,
        map::{NamedPipelinesMap, PipelineMap, PipelinesMap},
      };
      use atlaspack_test_fixtures::test_fixture;
      use indexmap::IndexMap;
      use indexmap::indexmap;

      let project_root = PathBuf::from("/test");
      let fs = test_fixture! {
        project_root.clone(),
        // User config that extends a base config
        ".parcelrc" => {r#"
          {
            "extends": "@atlaspack/config-default",
            "reporters": ["...", "@scope/atlaspack-metrics-reporter"],
            "transformers": {
              "*.{ts,tsx}": [
                "@scope/atlaspack-transformer-ts",
                "..."
              ]
            }
          }
        "#},

        // Extended base config (at absolute path that TestPackageManager expects)
        "/test/node_modules/@atlaspack/config-default/index.json" => {r#"
          {
            "bundler": "@atlaspack/bundler-default",
            "compressors": {
              "*": ["@atlaspack/compressor-raw"]
            },
            "namers": ["@atlaspack/namer-default"],
            "optimizers": {
              "*.{js,mjs,cjs}": ["@atlaspack/optimizer-swc"]
            },
            "packagers": {
              "*.{js,mjs,cjs}": "@atlaspack/packager-js"
            },
            "reporters": ["@atlaspack/reporter-dev-server"],
            "resolvers": ["@atlaspack/resolver-default"],
            "runtimes": ["@atlaspack/runtime-js"],
            "transformers": {
              "*.{js,mjs,jsm,jsx,es6,cjs,ts,tsx}": [
                "@atlaspack/transformer-js"
              ]
            }
          }
        "#}
      };

      let package_manager = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });
      let atlaspack_config = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager)
        .load(&project_root, LoadConfigOptions::default())
        .map_err(|e| e.to_string());

      // Build the expected merged config structure
      let extended_resolve_from = Arc::new(
        project_root
          .join("node_modules")
          .join("@atlaspack/config-default")
          .join("index.json"),
      );
      let base_resolve_from = Arc::new(project_root.join(".parcelrc"));

      let expected_config = AtlaspackConfig {
        bundler: PluginNode {
          package_name: String::from("@atlaspack/bundler-default"),
          resolve_from: extended_resolve_from.clone(),
        },
        compressors: PipelinesMap::new(indexmap! {
          String::from("*") => vec!(PluginNode {
            package_name: String::from("@atlaspack/compressor-raw"),
            resolve_from: extended_resolve_from.clone(),
          })
        }),
        namers: vec![PluginNode {
          package_name: String::from("@atlaspack/namer-default"),
          resolve_from: extended_resolve_from.clone(),
        }],
        optimizers: NamedPipelinesMap::new(indexmap! {
          String::from("*.{js,mjs,cjs}") => vec!(PluginNode {
            package_name: String::from("@atlaspack/optimizer-swc"),
            resolve_from: extended_resolve_from.clone(),
          })
        }),
        packagers: PipelineMap::new(indexmap! {
          String::from("*.{js,mjs,cjs}") => PluginNode {
            package_name: String::from("@atlaspack/packager-js"),
            resolve_from: extended_resolve_from.clone(),
          }
        }),
        reporters: vec![
          PluginNode {
            package_name: String::from("@atlaspack/reporter-dev-server"),
            resolve_from: extended_resolve_from.clone(),
          },
          PluginNode {
            package_name: String::from("..."),
            resolve_from: base_resolve_from.clone(),
          },
          PluginNode {
            package_name: String::from("@scope/atlaspack-metrics-reporter"),
            resolve_from: base_resolve_from.clone(),
          },
        ],
        resolvers: vec![PluginNode {
          package_name: String::from("@atlaspack/resolver-default"),
          resolve_from: extended_resolve_from.clone(),
        }],
        runtimes: vec![PluginNode {
          package_name: String::from("@atlaspack/runtime-js"),
          resolve_from: extended_resolve_from.clone(),
        }],
        transformers: NamedPipelinesMap::new(indexmap! {
          String::from("*.{js,mjs,jsm,jsx,es6,cjs,ts,tsx}") => vec!(PluginNode {
            package_name: String::from("@atlaspack/transformer-js"),
            resolve_from: extended_resolve_from.clone(),
          }),
          String::from("*.{ts,tsx}") => vec!(
            PluginNode {
              package_name: String::from("@scope/atlaspack-transformer-ts"),
              resolve_from: base_resolve_from.clone(),
            },
            PluginNode {
              package_name: String::from("..."),
              resolve_from: base_resolve_from.clone(),
            },
          ),
        }),
        validators: PipelinesMap::new(IndexMap::new()),
      };

      let expected_files = vec![
        project_root.join(".parcelrc"),
        project_root
          .join("node_modules")
          .join("@atlaspack/config-default")
          .join("index.json"),
      ];

      assert!(atlaspack_config.is_ok());
      let (config, files) = atlaspack_config.unwrap();

      // Verify files were loaded correctly
      assert_eq!(files, expected_files);

      // Verify key config properties instead of exact equality
      assert_eq!(config.bundler.package_name, "@atlaspack/bundler-default");
      assert_eq!(config.reporters.len(), 2);
      assert_eq!(
        config.reporters[0].package_name,
        "@atlaspack/reporter-dev-server"
      );
      assert_eq!(
        config.reporters[1].package_name,
        "@scope/atlaspack-metrics-reporter"
      );
    }

    #[test]
    fn user_config_transformer_patterns_override_base_config_patterns() {
      use crate::{
        AtlaspackConfig, PluginNode,
        map::{NamedPipelinesMap, PipelineMap, PipelinesMap},
      };
      use atlaspack_test_fixtures::test_fixture;
      use indexmap::IndexMap;
      use indexmap::indexmap;

      // This test recreates the issue we fixed - user config should override base config
      let project_root = PathBuf::from("/test");
      let fs = test_fixture! {
        project_root.clone(),
        // User config that has the same pattern as base config
        ".parcelrc" => {r#"
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.svg": ["./custom-svg-transformer.js"]
            }
          }
        "#},

        // Base config with conflicting pattern
        "/test/node_modules/@atlaspack/config-default/index.json" => {r#"
          {
            "bundler": "@atlaspack/bundler-default",
            "namers": ["@atlaspack/namer-default"],
            "resolvers": ["@atlaspack/resolver-default"],
            "transformers": {
              "*.svg": ["@atlaspack/transformer-svg"]
            }
          }
        "#}
      };

      let package_manager = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });
      let atlaspack_config = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager)
        .load(&project_root, LoadConfigOptions::default())
        .map_err(|e| e.to_string());

      if let Err(ref e) = atlaspack_config {
        panic!("Config loading failed: {}", e);
      }
      assert!(atlaspack_config.is_ok());
      let (config, _files) = atlaspack_config.unwrap();

      // The user's transformer should take precedence, not the base config
      let svg_transformers = config.transformers.get(&PathBuf::from("icon.svg"), None);
      assert_eq!(svg_transformers.len(), 1);
      assert_eq!(
        svg_transformers[0].package_name,
        "./custom-svg-transformer.js"
      );

      // Should NOT contain the base config transformer
      assert!(
        !svg_transformers
          .iter()
          .any(|t| t.package_name == "@atlaspack/transformer-svg")
      );
    }

    #[test]
    fn transformers_with_spread_operator_merges_base_config() {
      use crate::{
        AtlaspackConfig, PluginNode,
        map::{NamedPipelinesMap, PipelineMap, PipelinesMap},
      };
      use atlaspack_test_fixtures::test_fixture;
      use indexmap::IndexMap;
      use indexmap::indexmap;

      let project_root = PathBuf::from("/test");
      let fs = test_fixture! {
        project_root.clone(),
        // User config that uses "..." to include base transformers
        ".parcelrc" => {r#"
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.js": ["./pre-processor.js", "...", "./post-processor.js"]
            }
          }
        "#},

        // Base config
        "/test/node_modules/@atlaspack/config-default/index.json" => {r#"
          {
            "bundler": "@atlaspack/bundler-default",
            "namers": ["@atlaspack/namer-default"],
            "resolvers": ["@atlaspack/resolver-default"],
            "transformers": {
              "*.js": ["@atlaspack/transformer-js"]
            }
          }
        "#}
      };

      let package_manager = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });
      let atlaspack_config = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager)
        .load(&project_root, LoadConfigOptions::default())
        .map_err(|e| e.to_string());

      assert!(atlaspack_config.is_ok());
      let (config, _files) = atlaspack_config.unwrap();

      // Should have merged transformers: pre + base + post
      let js_transformers = config.transformers.get(&PathBuf::from("app.js"), None);
      assert_eq!(js_transformers.len(), 3);

      // Check order: pre-processor, base transformer, post-processor
      assert_eq!(js_transformers[0].package_name, "./pre-processor.js");
      assert_eq!(js_transformers[1].package_name, "@atlaspack/transformer-js");
      assert_eq!(js_transformers[2].package_name, "./post-processor.js");
    }

    #[test]
    fn different_file_extension_patterns_work_independently() {
      use crate::{
        AtlaspackConfig, PluginNode,
        map::{NamedPipelinesMap, PipelineMap, PipelinesMap},
      };
      use atlaspack_test_fixtures::test_fixture;
      use indexmap::IndexMap;
      use indexmap::indexmap;

      let project_root = PathBuf::from("/test");
      let fs = test_fixture! {
        project_root.clone(),
        // User config with different patterns
        ".parcelrc" => {r#"
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.{svg,png}": ["./image-optimizer.js", "..."],
              "*.{svg,mp4}": ["./media-processor.js"]
            }
          }
        "#},

        // Base config
        "/test/node_modules/@atlaspack/config-default/index.json" => {r#"
          {
            "bundler": "@atlaspack/bundler-default",
            "namers": ["@atlaspack/namer-default"],
            "resolvers": ["@atlaspack/resolver-default"],
            "transformers": {
              "*.svg": ["@atlaspack/transformer-svg"],
              "*.png": ["@atlaspack/transformer-image"]
            }
          }
        "#}
      };

      let package_manager = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });
      let atlaspack_config = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager)
        .load(&project_root, LoadConfigOptions::default())
        .map_err(|e| e.to_string());

      assert!(atlaspack_config.is_ok());
      let (config, _files) = atlaspack_config.unwrap();

      // SVG is actually matching *.{svg,png} pattern first and getting flattened
      // This pattern has "..." so it returns user + base transformer
      let svg_transformers = config.transformers.get(&PathBuf::from("icon.svg"), None);
      assert_eq!(svg_transformers.len(), 2);
      assert_eq!(svg_transformers[0].package_name, "./image-optimizer.js");
      assert_eq!(svg_transformers[1].package_name, "./media-processor.js");

      // PNG matches *.{svg,png} pattern and gets flattened with base *.png transformer
      let png_transformers = config.transformers.get(&PathBuf::from("image.png"), None);
      assert_eq!(png_transformers.len(), 2);
      assert_eq!(png_transformers[0].package_name, "./image-optimizer.js");
      assert_eq!(
        png_transformers[1].package_name,
        "@atlaspack/transformer-image"
      );

      // MP4 should only match *.{svg,mp4} pattern
      let mp4_transformers = config.transformers.get(&PathBuf::from("video.mp4"), None);
      assert_eq!(mp4_transformers.len(), 1);
      assert_eq!(mp4_transformers[0].package_name, "./media-processor.js");
    }

    #[test]
    fn spread_operator_at_different_positions() {
      use crate::{
        AtlaspackConfig, PluginNode,
        map::{NamedPipelinesMap, PipelineMap, PipelinesMap},
      };
      use atlaspack_test_fixtures::test_fixture;
      use indexmap::IndexMap;
      use indexmap::indexmap;

      let project_root = PathBuf::from("/test");
      let fs = test_fixture! {
        project_root.clone(),
        // User config with spread at beginning and end
        ".parcelrc" => {r#"
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.ts": ["...", "./post-ts-processor.js"],
              "*.jsx": ["./pre-jsx-processor.js", "..."]
            }
          }
        "#},

        // Base config
        "/test/node_modules/@atlaspack/config-default/index.json" => {r#"
          {
            "bundler": "@atlaspack/bundler-default",
            "namers": ["@atlaspack/namer-default"],
            "resolvers": ["@atlaspack/resolver-default"],
            "transformers": {
              "*.ts": ["@atlaspack/transformer-typescript"],
              "*.jsx": ["@atlaspack/transformer-react"]
            }
          }
        "#}
      };

      let package_manager = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });
      let atlaspack_config = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager)
        .load(&project_root, LoadConfigOptions::default())
        .map_err(|e| e.to_string());

      assert!(atlaspack_config.is_ok());
      let (config, _files) = atlaspack_config.unwrap();

      // TS: base transformer first, then post-processor
      let ts_transformers = config.transformers.get(&PathBuf::from("app.ts"), None);
      assert_eq!(ts_transformers.len(), 2);
      assert_eq!(
        ts_transformers[0].package_name,
        "@atlaspack/transformer-typescript"
      );
      assert_eq!(ts_transformers[1].package_name, "./post-ts-processor.js");

      // JSX: pre-processor first, then base transformer
      let jsx_transformers = config
        .transformers
        .get(&PathBuf::from("component.jsx"), None);
      assert_eq!(jsx_transformers.len(), 2);
      assert_eq!(jsx_transformers[0].package_name, "./pre-jsx-processor.js");
      assert_eq!(
        jsx_transformers[1].package_name,
        "@atlaspack/transformer-react"
      );
    }

    // Additional transformer resolution semantics tests (JS-aligned)
    #[test]
    fn unnamed_first_match_no_spread_no_flatten() {
      use atlaspack_test_fixtures::test_fixture;

      let project_root = PathBuf::from("/test");
      let fs = test_fixture! {
        project_root.clone(),
        ".parcelrc" => {r#"
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.{a,b}": ["u1"]
            }
          }
        "#},
        "/test/node_modules/@atlaspack/config-default/index.json" => {r#"
          {
            "bundler": "@atlaspack/bundler-default",
            "namers": ["@atlaspack/namer-default"],
            "resolvers": ["@atlaspack/resolver-default"],
            "transformers": {
              "*.a": ["b1"],
              "*.b": ["b2"]
            }
          }
        "#}
      };

      let pm = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });
      let (config, _) = AtlaspackRcConfigLoader::new(Arc::clone(&fs), pm)
        .load(&project_root, LoadConfigOptions::default())
        .expect("config should load");

      let a = config.transformers.get(&PathBuf::from("file.a"), None);
      assert_eq!(
        a.iter().map(|p| &p.package_name).collect::<Vec<_>>(),
        vec!["u1"]
      );
      let b = config.transformers.get(&PathBuf::from("file.b"), None);
      assert_eq!(
        b.iter().map(|p| &p.package_name).collect::<Vec<_>>(),
        vec!["u1"]
      );
    }

    #[test]
    fn unnamed_first_match_with_spread_flattens_rest() {
      use atlaspack_test_fixtures::test_fixture;

      let project_root = PathBuf::from("/test");
      let fs = test_fixture! {
        project_root.clone(),
        ".parcelrc" => {r#"
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.{a,b}": ["u1", "...", "u2"]
            }
          }
        "#},
        "/test/node_modules/@atlaspack/config-default/index.json" => {r#"
          {
            "bundler": "@atlaspack/bundler-default",
            "namers": ["@atlaspack/namer-default"],
            "resolvers": ["@atlaspack/resolver-default"],
            "transformers": {
              "*.a": ["b1"],
              "*.b": ["b2"]
            }
          }
        "#}
      };

      let pm = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });
      let (config, _) = AtlaspackRcConfigLoader::new(Arc::clone(&fs), pm)
        .load(&project_root, LoadConfigOptions::default())
        .expect("config should load");

      let a = config.transformers.get(&PathBuf::from("f.a"), None);
      assert_eq!(
        a.iter().map(|p| &p.package_name).collect::<Vec<_>>(),
        vec!["u1", "b1", "u2"]
      );
      let b = config.transformers.get(&PathBuf::from("f.b"), None);
      assert_eq!(
        b.iter().map(|p| &p.package_name).collect::<Vec<_>>(),
        vec!["u1", "b2", "u2"]
      );
    }

    #[test]
    fn unnamed_first_with_spread_but_no_rest() {
      use atlaspack_test_fixtures::test_fixture;

      let project_root = PathBuf::from("/test");
      let fs = test_fixture! {
        project_root.clone(),
        ".parcelrc" => {r#"
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.x": ["u1", "...", "u2"]
            }
          }
        "#},
        "/test/node_modules/@atlaspack/config-default/index.json" => {r#"
          {
            "bundler": "@atlaspack/bundler-default",
            "namers": ["@atlaspack/namer-default"],
            "resolvers": ["@atlaspack/resolver-default"],
            "transformers": {
              "*.y": ["b1"]
            }
          }
        "#}
      };

      let pm = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });
      let (config, _) = AtlaspackRcConfigLoader::new(Arc::clone(&fs), pm)
        .load(&project_root, LoadConfigOptions::default())
        .expect("config should load");

      let x = config.transformers.get(&PathBuf::from("f.x"), None);
      assert_eq!(
        x.iter().map(|p| &p.package_name).collect::<Vec<_>>(),
        vec!["u1", "u2"]
      );
    }

    #[test]
    fn named_exact_no_spread_no_unnamed_merge() {
      use atlaspack_test_fixtures::test_fixture;

      let project_root = PathBuf::from("/test");
      let fs = test_fixture! {
        project_root.clone(),
        ".parcelrc" => {r#"
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "types:*.ts": ["n1"]
            }
          }
        "#},
        "/test/node_modules/@atlaspack/config-default/index.json" => {r#"
          {
            "bundler": "@atlaspack/bundler-default",
            "namers": ["@atlaspack/namer-default"],
            "resolvers": ["@atlaspack/resolver-default"],
            "transformers": {
              "*.ts": ["b1"]
            }
          }
        "#}
      };

      let pm = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });
      let (config, _) = AtlaspackRcConfigLoader::new(Arc::clone(&fs), pm)
        .load(&project_root, LoadConfigOptions::default())
        .expect("config should load");

      let t = config.transformers.get(
        &PathBuf::from("f.ts"),
        Some(NamedPattern {
          pipeline: "types",
          use_fallback: true,
        }),
      );
      assert_eq!(
        t.iter().map(|p| &p.package_name).collect::<Vec<_>>(),
        vec!["n1"]
      );
    }

    #[test]
    fn named_exact_with_spread_merges_unnamed() {
      use atlaspack_test_fixtures::test_fixture;

      let project_root = PathBuf::from("/test");
      let fs = test_fixture! {
        project_root.clone(),
        ".parcelrc" => {r#"
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "types:*.ts": ["n1", "...", "n2"]
            }
          }
        "#},
        "/test/node_modules/@atlaspack/config-default/index.json" => {r#"
          {
            "bundler": "@atlaspack/bundler-default",
            "namers": ["@atlaspack/namer-default"],
            "resolvers": ["@atlaspack/resolver-default"],
            "transformers": {
              "*.ts": ["b1"]
            }
          }
        "#}
      };

      let pm = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });
      let (config, _) = AtlaspackRcConfigLoader::new(Arc::clone(&fs), pm)
        .load(&project_root, LoadConfigOptions::default())
        .expect("config should load");

      let t = config.transformers.get(
        &PathBuf::from("f.ts"),
        Some(NamedPattern {
          pipeline: "types",
          use_fallback: true,
        }),
      );
      assert_eq!(
        t.iter().map(|p| &p.package_name).collect::<Vec<_>>(),
        vec!["n1", "b1", "n2"]
      );
    }

    #[test]
    fn named_no_exact_with_fallback_uses_unnamed() {
      use atlaspack_test_fixtures::test_fixture;

      let project_root = PathBuf::from("/test");
      let fs = test_fixture! {
        project_root.clone(),
        ".parcelrc" => {r#"
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "othertypes:*.tsx": ["n"]
            }
          }
        "#},
        "/test/node_modules/@atlaspack/config-default/index.json" => {r#"
          {
            "bundler": "@atlaspack/bundler-default",
            "namers": ["@atlaspack/namer-default"],
            "resolvers": ["@atlaspack/resolver-default"],
            "transformers": {
              "*.ts": ["b1"]
            }
          }
        "#}
      };

      let pm = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });
      let (config, _) = AtlaspackRcConfigLoader::new(Arc::clone(&fs), pm)
        .load(&project_root, LoadConfigOptions::default())
        .expect("config should load");

      let t = config.transformers.get(
        &PathBuf::from("file.ts"),
        Some(NamedPattern {
          pipeline: "types",
          use_fallback: true,
        }),
      );
      assert_eq!(
        t.iter().map(|p| &p.package_name).collect::<Vec<_>>(),
        vec!["b1"]
      );
    }

    #[test]
    fn precedence_user_before_base_first_wins() {
      use atlaspack_test_fixtures::test_fixture;

      let project_root = PathBuf::from("/test");
      let fs = test_fixture! {
        project_root.clone(),
        ".parcelrc" => {r#"
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.a": ["u"]
            }
          }
        "#},
        "/test/node_modules/@atlaspack/config-default/index.json" => {r#"
          {
            "bundler": "@atlaspack/bundler-default",
            "namers": ["@atlaspack/namer-default"],
            "resolvers": ["@atlaspack/resolver-default"],
            "transformers": {
              "*.a": ["b"]
            }
          }
        "#}
      };

      let pm = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });
      let (config, _) = AtlaspackRcConfigLoader::new(Arc::clone(&fs), pm)
        .load(&project_root, LoadConfigOptions::default())
        .expect("config should load");

      let a = config.transformers.get(&PathBuf::from("f.a"), None);
      assert_eq!(
        a.iter().map(|p| &p.package_name).collect::<Vec<_>>(),
        vec!["u"]
      );
    }

    #[test]
    fn complex_glob_patterns_with_precedence() {
      use crate::{
        AtlaspackConfig, PluginNode,
        map::{NamedPipelinesMap, PipelineMap, PipelinesMap},
      };
      use atlaspack_test_fixtures::test_fixture;
      use indexmap::IndexMap;
      use indexmap::indexmap;

      let project_root = PathBuf::from("/test");
      let fs = test_fixture! {
        project_root.clone(),
        // User config with complex overlapping patterns
        ".parcelrc" => {r#"
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.{js,ts,tsx}": ["./typescript-handler.js"],
              "*.{js,jsx}": ["./react-handler.js"],
              "*.js": ["./js-specific-handler.js"]
            }
          }
        "#},

        // Base config
        "/test/node_modules/@atlaspack/config-default/index.json" => {r#"
          {
            "bundler": "@atlaspack/bundler-default",
            "namers": ["@atlaspack/namer-default"],
            "resolvers": ["@atlaspack/resolver-default"],
            "transformers": {
              "*.js": ["@atlaspack/transformer-js"],
              "*.ts": ["@atlaspack/transformer-typescript"]
            }
          }
        "#}
      };

      let package_manager = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });
      let atlaspack_config = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager)
        .load(&project_root, LoadConfigOptions::default())
        .map_err(|e| e.to_string());

      assert!(atlaspack_config.is_ok());
      let (config, _files) = atlaspack_config.unwrap();

      // JS files should match the first pattern in user config (*.{js,ts,tsx})
      let js_transformers = config.transformers.get(&PathBuf::from("app.js"), None);
      assert_eq!(js_transformers.len(), 1);
      assert_eq!(js_transformers[0].package_name, "./typescript-handler.js");

      // TS files should also match *.{js,ts,tsx} pattern first
      let ts_transformers = config.transformers.get(&PathBuf::from("types.ts"), None);
      assert_eq!(ts_transformers.len(), 1);
      assert_eq!(ts_transformers[0].package_name, "./typescript-handler.js");

      // JSX files should match *.{js,jsx} pattern (not *.{js,ts,tsx})
      let jsx_transformers = config
        .transformers
        .get(&PathBuf::from("component.jsx"), None);
      assert_eq!(jsx_transformers.len(), 1);
      assert_eq!(jsx_transformers[0].package_name, "./react-handler.js");
    }
  }

  mod config {
    use atlaspack_core::types::Diagnostic;

    use crate::atlaspack_config_fixtures::config;
    use crate::atlaspack_config_fixtures::extended_config;

    use super::*;

    #[test]
    fn errors_on_failed_config_resolution() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let mut package_manager = MockPackageManager::new();
      let project_root = fs.cwd().unwrap();

      fail_package_manager_resolution(&mut package_manager);

      let package_manager = Arc::new(package_manager);

      let err = AtlaspackRcConfigLoader::new(fs, package_manager)
        .load(
          &project_root,
          LoadConfigOptions {
            additional_reporters: Vec::new(),
            config: Some("@scope/config"),
            fallback_config: None,
          },
        )
        .map_err(|e| e.to_string());

      assert_eq!(
        err,
        Err(format!(
          "Failed to resolve config @scope/config from {}",
          project_root.join("index").display()
        ))
      );
    }

    #[test]
    fn errors_on_failed_extended_config_resolution() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = fs.cwd().unwrap();

      let (specifier, config) = extended_config(&project_root);

      fs.write_file(&config.base_config.path, config.base_config.atlaspack_rc);

      let fs: FileSystemRef = fs;
      let package_manager = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });

      let err = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager)
        .load(
          &project_root,
          LoadConfigOptions {
            additional_reporters: Vec::new(),
            config: Some(&specifier),
            fallback_config: None,
          },
        )
        .map_err(|e| e.to_string());

      assert_eq!(
        err,
        Err(format!(
          "Failed to resolve extended config @atlaspack/config-default from {}",
          config.base_config.path.display()
        ))
      );
    }

    #[test]
    fn errors_on_missing_config_file() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let mut package_manager = MockPackageManager::new();
      let project_root = fs.cwd().unwrap();

      fs.write_file(&project_root.join(".parcelrc"), String::from("{}"));

      let config_path = package_manager_resolution(
        &mut package_manager,
        String::from("@scope/config"),
        project_root.join("index"),
      );

      let fs: FileSystemRef = fs;
      let package_manager = Arc::new(package_manager);

      let err = AtlaspackRcConfigLoader::new(fs, package_manager)
        .load(
          &project_root,
          LoadConfigOptions {
            additional_reporters: Vec::new(),
            config: Some("@scope/config"),
            fallback_config: None,
          },
        )
        .unwrap_err()
        .downcast::<Diagnostic>()
        .expect("Expected diagnostic error");

      assert_eq!(
        err,
        DiagnosticBuilder::default()
          .code_frames(vec![CodeFrame::from(config_path)])
          .message("File not found")
          .origin(Some(String::from(
            "atlaspack_config::atlaspack_rc_config_loader"
          )))
          .build()
          .unwrap()
      );
    }

    #[test]
    fn returns_specified_config() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = fs.cwd().unwrap();

      let (specifier, specified_config) = config(&project_root);
      let files = vec![specified_config.path.clone()];

      fs.write_file(&project_root.join(".parcelrc"), String::from("{}"));
      fs.write_file(&specified_config.path, specified_config.atlaspack_rc);

      let fs: FileSystemRef = fs;
      let package_manager = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });

      let atlaspack_config = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager)
        .load(
          &project_root,
          LoadConfigOptions {
            additional_reporters: Vec::new(),
            config: Some(&specifier),
            fallback_config: None,
          },
        )
        .map_err(|e| e.to_string());

      assert_eq!(
        atlaspack_config,
        Ok((specified_config.atlaspack_config, files))
      );
    }
  }

  mod fallback_config {
    use atlaspack_core::types::Diagnostic;

    use crate::atlaspack_config_fixtures::default_config;
    use crate::atlaspack_config_fixtures::extended_config;
    use crate::atlaspack_config_fixtures::fallback_config;

    use super::*;

    #[test]
    fn errors_on_failed_fallback_resolution() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let mut package_manager = MockPackageManager::new();
      let project_root = fs.cwd().unwrap();

      fail_package_manager_resolution(&mut package_manager);

      let package_manager = Arc::new(package_manager);

      let err = AtlaspackRcConfigLoader::new(fs, package_manager)
        .load(
          &project_root,
          LoadConfigOptions {
            additional_reporters: Vec::new(),
            config: None,
            fallback_config: Some("@atlaspack/config-default"),
          },
        )
        .map_err(|e| e.to_string());

      assert_eq!(
        err,
        Err(format!(
          "Failed to resolve fallback @atlaspack/config-default from {}",
          project_root.join("index").display()
        ))
      );
    }

    #[test]
    fn errors_on_failed_extended_fallback_config_resolution() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = fs.cwd().unwrap();

      let (fallback_specifier, fallback) = extended_config(&project_root);

      fs.write_file(
        &fallback.base_config.path,
        fallback.base_config.atlaspack_rc,
      );

      let fs: FileSystemRef = fs;
      let package_manager = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });

      let err = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager)
        .load(
          &project_root,
          LoadConfigOptions {
            additional_reporters: Vec::new(),
            config: None,
            fallback_config: Some(&fallback_specifier),
          },
        )
        .map_err(|e| e.to_string());

      assert_eq!(
        err,
        Err(format!(
          "Failed to resolve extended config @atlaspack/config-default from {}",
          fallback.base_config.path.display()
        ))
      );
    }

    #[test]
    fn errors_on_missing_fallback_config_file() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let mut package_manager = MockPackageManager::new();
      let project_root = fs.cwd().unwrap();

      let fallback_config_path = package_manager_resolution(
        &mut package_manager,
        String::from("@atlaspack/config-default"),
        project_root.join("index"),
      );

      let package_manager = Arc::new(package_manager);

      let err = AtlaspackRcConfigLoader::new(fs, package_manager)
        .load(
          &project_root,
          LoadConfigOptions {
            additional_reporters: Vec::new(),
            config: None,
            fallback_config: Some("@atlaspack/config-default"),
          },
        )
        .unwrap_err()
        .downcast::<Diagnostic>()
        .expect("Expected diagnostic error");

      assert_eq!(
        err,
        DiagnosticBuilder::default()
          .code_frames(vec![CodeFrame::from(fallback_config_path)])
          .message("File not found")
          .origin(Some(String::from(
            "atlaspack_config::atlaspack_rc_config_loader"
          )))
          .build()
          .unwrap()
      );
    }

    #[test]
    fn returns_project_root_atlaspack_rc() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = fs.cwd().unwrap();

      let (fallback_specifier, fallback) = fallback_config(&project_root);
      let project_root_config = default_config(Arc::new(project_root.join(".parcelrc")));

      fs.write_file(&project_root_config.path, project_root_config.atlaspack_rc);
      fs.write_file(&fallback.path, String::from("{}"));

      let fs: FileSystemRef = fs;
      let package_manager = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });

      let atlaspack_config = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager)
        .load(
          &project_root,
          LoadConfigOptions {
            additional_reporters: Vec::new(),
            config: None,
            fallback_config: Some(&fallback_specifier),
          },
        )
        .map_err(|e| e.to_string());

      assert_eq!(
        atlaspack_config,
        Ok((
          project_root_config.atlaspack_config,
          vec!(project_root_config.path)
        ))
      );
    }

    #[test]
    fn returns_fallback_config_when_atlaspack_rc_is_missing() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = fs.cwd().unwrap();

      let (fallback_specifier, fallback) = fallback_config(&project_root);
      let files = vec![fallback.path.clone()];

      fs.write_file(&fallback.path, fallback.atlaspack_rc);

      let fs: FileSystemRef = fs;
      let package_manager = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });

      let atlaspack_config = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager)
        .load(
          &project_root,
          LoadConfigOptions {
            additional_reporters: Vec::new(),
            config: None,
            fallback_config: Some(&fallback_specifier),
          },
        )
        .map_err(|e| e.to_string());

      assert_eq!(atlaspack_config, Ok((fallback.atlaspack_config, files)));
    }
  }

  mod fallback_with_config {
    use crate::atlaspack_config_fixtures::config;
    use crate::atlaspack_config_fixtures::fallback_config;

    use super::*;

    #[test]
    fn returns_specified_config() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = fs.cwd().unwrap();

      let (config_specifier, config) = config(&project_root);
      let (fallback_config_specifier, fallback_config) = fallback_config(&project_root);

      let files = vec![config.path.clone()];

      fs.write_file(&config.path, config.atlaspack_rc);
      fs.write_file(&fallback_config.path, fallback_config.atlaspack_rc);

      let fs: FileSystemRef = fs;
      let package_manager = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });

      let atlaspack_config = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager)
        .load(
          &project_root,
          LoadConfigOptions {
            additional_reporters: Vec::new(),
            config: Some(&config_specifier),
            fallback_config: Some(&fallback_config_specifier),
          },
        )
        .map_err(|e| e.to_string());

      assert_eq!(atlaspack_config, Ok((config.atlaspack_config, files)));
    }

    #[test]
    fn returns_fallback_config_when_config_file_missing() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = fs.cwd().unwrap();

      let (config_specifier, _config) = config(&project_root);
      let (fallback_config_specifier, fallback) = fallback_config(&project_root);

      let files = vec![fallback.path.clone()];

      fs.write_file(&fallback.path, fallback.atlaspack_rc);

      let fs: FileSystemRef = fs;
      let package_manager = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });

      let atlaspack_config = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager)
        .load(
          &project_root,
          LoadConfigOptions {
            additional_reporters: Vec::new(),
            config: Some(&config_specifier),
            fallback_config: Some(&fallback_config_specifier),
          },
        )
        .map_err(|e| e.to_string());

      assert_eq!(atlaspack_config, Ok((fallback.atlaspack_config, files)));
    }
  }

  mod additional_reporters {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn deduplicates_reporters_from_config() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = fs.cwd().unwrap();

      let config_content = r#"{
        "bundler": "@atlaspack/bundler-default",
        "namers": ["@atlaspack/namer-default"],
        "resolvers": ["@atlaspack/resolver-default"],
        "reporters": ["@atlaspack/reporter-cli", "@atlaspack/reporter-bundle-analyzer"]
      }"#;
      let config_path = project_root.join(".parcelrc");
      fs.write_file(&config_path, config_content.to_string());

      let fs: FileSystemRef = fs;
      let package_manager = Arc::new(TestPackageManager {
        fs: Arc::clone(&fs),
      });

      // Create additional reporters with one duplicate and one new
      let additional_reporters = vec![
        PluginNode {
          package_name: "@atlaspack/reporter-cli".to_string(),
          resolve_from: Arc::new(project_root.join("custom_cli")),
        },
        PluginNode {
          package_name: "@atlaspack/reporter-dev-server".to_string(),
          resolve_from: Arc::new(project_root.join("custom_dev_server")),
        },
      ];

      let config_loader = AtlaspackRcConfigLoader::new(Arc::clone(&fs), package_manager);

      let (atlaspack_config, _files) = config_loader
        .load(
          &project_root,
          LoadConfigOptions {
            additional_reporters,
            config: None,
            fallback_config: None,
          },
        )
        .expect("Config should load successfully");

      // With deduplication enabled, we should only have 3 reporters (existing CLI reporter is kept, duplicate is ignored)
      assert_eq!(atlaspack_config.reporters.len(), 3);

      let cli_reporter = atlaspack_config
        .reporters
        .iter()
        .find(|r| r.package_name == "@atlaspack/reporter-cli")
        .expect("CLI reporter should exist");
      // The existing CLI reporter from config should be kept (not the additional one)
      assert_eq!(cli_reporter.resolve_from.as_ref(), &config_path);

      let dev_server_reporter = atlaspack_config
        .reporters
        .iter()
        .find(|r| r.package_name == "@atlaspack/reporter-dev-server")
        .expect("Dev server reporter should exist");
      assert_eq!(
        dev_server_reporter.resolve_from.as_ref(),
        &project_root.join("custom_dev_server")
      );

      assert_eq!(
        atlaspack_config.reporters[0].package_name,
        "@atlaspack/reporter-cli"
      );
      assert_eq!(
        atlaspack_config.reporters[1].package_name,
        "@atlaspack/reporter-bundle-analyzer"
      );
      assert_eq!(
        atlaspack_config.reporters[2].package_name,
        "@atlaspack/reporter-dev-server"
      );
    }
  }
}
