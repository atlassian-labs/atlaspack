use std::path::Path;
use std::{borrow::Cow, hash::Hasher, path::PathBuf};

use anyhow::anyhow;
use atlaspack_filesystem::FileSystemRef;
use atlaspack_resolver::{FileCreateInvalidation, Invalidations, Resolution, SpecifierType};

use crate::{DevDep, PackageManager};

pub struct NodePackageManager<'a> {
  project_root: PathBuf,
  dev_dep_cache: atlaspack_dev_dep_resolver::Cache,
  resolver: atlaspack_resolver::Resolver<'a>,
}

impl NodePackageManager<'_> {
  pub fn new(project_root: PathBuf, fs: FileSystemRef) -> Self {
    Self {
      project_root: project_root.clone(),
      dev_dep_cache: atlaspack_dev_dep_resolver::Cache::default(),
      resolver: atlaspack_resolver::Resolver::node(
        Cow::Owned(project_root),
        atlaspack_resolver::CacheCow::Owned(atlaspack_resolver::Cache::new(fs)),
      ),
    }
  }
}

impl PackageManager for NodePackageManager<'_> {
  fn resolve(&self, specifier: &str, from: &std::path::Path) -> anyhow::Result<crate::Resolution> {
    let res = self.resolver.resolve(specifier, from, SpecifierType::Cjs);

    match res.result? {
      (Resolution::Path(pathbuf), _invalidations) => Ok(crate::Resolution { resolved: pathbuf }),
      other_case => Err(anyhow!("Unexpected resolution result: {:?}", other_case)),
    }
  }

  #[tracing::instrument(level = "debug", skip(self, resolve_from), ret)]
  fn resolve_dev_dependency(
    &self,
    package_name: &str,
    resolve_from: &std::path::Path,
    ignore_startup_invalidations: bool,
  ) -> anyhow::Result<DevDep> {
    let crate::Resolution { resolved } = self.resolve(package_name, resolve_from)?;

    // Build ESM graph for the main transformer package
    let all_invalidations = match atlaspack_dev_dep_resolver::build_esm_graph(
      &resolved,
      &self.project_root,
      &self.resolver.cache,
      &self.dev_dep_cache,
    ) {
      Err(err) => {
        tracing::warn!(
          "Error while building dev dependency graph for '{}': {:?}",
          package_name,
          err
        );
        return Ok(DevDep {
          file_path: resolved,
          hash: None,
        });
      }
      Ok(result) => result,
    };

    // Read additional dev deps from the consumer's package.json.
    // These are packages that the transformer dynamically loads at runtime
    // (e.g., a custom resolver) which the static ESM graph walk cannot discover.
    let additional_deps = self.read_additional_dev_deps(package_name, resolve_from);

    for dep in &additional_deps {
      match self.resolve(dep, resolve_from) {
        Ok(crate::Resolution { resolved: dep_path }) => {
          match atlaspack_dev_dep_resolver::build_esm_graph(
            &dep_path,
            &self.project_root,
            &self.resolver.cache,
            &self.dev_dep_cache,
          ) {
            Ok(dep_invalidations) => {
              tracing::debug!(
                "Built ESM graph for additional dev dep '{}' (resolved to {:?})",
                dep,
                dep_path
              );
              all_invalidations.extend(&dep_invalidations);
            }
            Err(err) => {
              tracing::warn!(
                "Error building ESM graph for additional dev dep '{}': {:?}",
                dep,
                err
              );
            }
          }
        }
        Err(err) => {
          tracing::warn!(
            "Could not resolve additional dev dep '{}' from {:?}: {:?}",
            dep,
            resolve_from,
            err
          );
        }
      }
    }

    Ok(DevDep {
      file_path: resolved,
      hash: self
        .hash_invalidations(&all_invalidations, ignore_startup_invalidations)
        .ok(),
    })
  }
}

impl NodePackageManager<'_> {
  /// Reads `additionalDevDeps` from the consumer's package.json for a given plugin.
  ///
  /// The consumer (e.g. Confluence) can declare additional packages that a transformer
  /// dynamically loads at runtime. These are specified in package.json under the
  /// `"atlaspack"` top-level key, namespaced by plugin package name:
  ///
  /// ```json
  /// {
  ///   "atlaspack": {
  ///     "@atlaspack/transformer-compiled": {
  ///       "additionalDevDeps": ["@devtools/compiled-resolver"]
  ///     }
  ///   }
  /// }
  /// ```
  ///
  /// We use the `"atlaspack"` namespace to avoid collisions with Atlaspack's own
  /// `getConfigFrom` API, which reads plugin config from `package.json` using the
  /// plugin's package name as a top-level key.
  ///
  /// The `resolve_from` path points to the `.parcelrc` file. We look for `package.json`
  /// in the same directory (the project root).
  fn read_additional_dev_deps(&self, package_name: &str, resolve_from: &Path) -> Vec<String> {
    let package_json_path = match resolve_from.parent() {
      Some(dir) => dir.join("package.json"),
      None => return Vec::new(),
    };

    let contents = match self.resolver.cache.fs.read_to_string(&package_json_path) {
      Ok(contents) => contents,
      Err(_) => return Vec::new(),
    };

    let json: serde_json::Value = match serde_json::from_str(&contents) {
      Ok(json) => json,
      Err(err) => {
        tracing::warn!(
          "Failed to parse {:?} for additionalDevDeps: {:?}",
          package_json_path,
          err
        );
        return Vec::new();
      }
    };

    let additional_deps = json
      .get("atlaspack")
      .and_then(|atlaspack_config| atlaspack_config.get(package_name))
      .and_then(|plugin_config| plugin_config.get("additionalDevDeps"))
      .and_then(|deps| deps.as_array())
      .map(|deps| {
        deps
          .iter()
          .filter_map(|dep| dep.as_str().map(String::from))
          .collect::<Vec<_>>()
      })
      .unwrap_or_default();

    if !additional_deps.is_empty() {
      tracing::info!(
        "Found additionalDevDeps for '{}' in {:?}: {:?}",
        package_name,
        package_json_path,
        additional_deps
      );
    }

    additional_deps
  }

  #[tracing::instrument(level = "debug", skip_all)]
  fn hash_invalidations(
    &self,
    invalidations: &Invalidations,
    ignore_startup_invalidations: bool,
  ) -> anyhow::Result<u64> {
    if !ignore_startup_invalidations
      && invalidations
        .invalidate_on_startup
        .load(std::sync::atomic::Ordering::Relaxed)
    {
      tracing::warn!("Start-up invalidation");
      return Err(anyhow!(
        "Dev dependency is not cacheable because it invalidates on startup"
      ));
    }

    for invalidation in invalidations.invalidate_on_file_create.read().iter() {
      match invalidation {
        FileCreateInvalidation::FileName { file_name, above } => {
          // We ignore file name above invalidations for hashes as we don't
          // cache the resolutions
          tracing::trace!("File create invalidation {} above {:?}", file_name, above);
        }
        FileCreateInvalidation::Path(file_path) => {
          // We ignore file name create invalidations because we don't cache the
          // resolution
          tracing::trace!("File create invalidation: {:?}", file_path);
        }
        FileCreateInvalidation::Glob(glob) => {
          tracing::warn!("Glob create invalidation: {:?}", glob);
          if !ignore_startup_invalidations {
            return Err(anyhow!(
              "Dev dependency is not cacheable because it invalidates on file create (glob): {:?}",
              glob
            ));
          }
        }
      }
    }

    let mut hasher = atlaspack_core::hash::IdentifierHasher::new();

    // TODO: We should run this in parralel
    for file_path in invalidations.invalidate_on_file_change.read().iter() {
      let file_contents = self.resolver.cache.fs.read(file_path)?;
      hasher.write(&file_contents);
    }

    Ok(hasher.finish())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;
  use pretty_assertions::assert_eq;
  use std::sync::Arc;

  fn create_package_manager<'a>(
    fs: &Arc<InMemoryFileSystem>,
    project_root: &str,
  ) -> NodePackageManager<'a> {
    NodePackageManager::new(PathBuf::from(project_root), fs.clone() as FileSystemRef)
  }

  #[test]
  fn test_read_additional_dev_deps_returns_deps_when_present() {
    let fs = Arc::new(InMemoryFileSystem::default());
    fs.write_file(
      &PathBuf::from("/project/package.json"),
      serde_json::json!({
        "name": "my-app",
        "atlaspack": {
          "@atlaspack/transformer-compiled": {
            "additionalDevDeps": ["@devtools/compiled-resolver", "@custom/other-dep"]
          }
        }
      })
      .to_string(),
    );

    let pm = create_package_manager(&fs, "/project");
    let resolve_from = Path::new("/project/.parcelrc");

    let deps = pm.read_additional_dev_deps("@atlaspack/transformer-compiled", resolve_from);
    assert_eq!(
      deps,
      vec![
        "@devtools/compiled-resolver".to_string(),
        "@custom/other-dep".to_string(),
      ]
    );
  }

  #[test]
  fn test_read_additional_dev_deps_returns_empty_when_no_config() {
    let fs = Arc::new(InMemoryFileSystem::default());
    fs.write_file(
      &PathBuf::from("/project/package.json"),
      serde_json::json!({
        "name": "my-app",
        "dependencies": {}
      })
      .to_string(),
    );

    let pm = create_package_manager(&fs, "/project");
    let resolve_from = Path::new("/project/.parcelrc");

    let deps = pm.read_additional_dev_deps("@atlaspack/transformer-compiled", resolve_from);
    assert_eq!(deps, Vec::<String>::new());
  }

  #[test]
  fn test_read_additional_dev_deps_returns_empty_when_no_package_json() {
    let fs = Arc::new(InMemoryFileSystem::default());
    let pm = create_package_manager(&fs, "/project");
    let resolve_from = Path::new("/project/.parcelrc");

    let deps = pm.read_additional_dev_deps("@atlaspack/transformer-compiled", resolve_from);
    assert_eq!(deps, Vec::<String>::new());
  }

  #[test]
  fn test_read_additional_dev_deps_returns_empty_when_no_additional_dev_deps_key() {
    let fs = Arc::new(InMemoryFileSystem::default());
    fs.write_file(
      &PathBuf::from("/project/package.json"),
      serde_json::json!({
        "name": "my-app",
        "atlaspack": {
          "@atlaspack/transformer-compiled": {
            "someOtherConfig": true
          }
        }
      })
      .to_string(),
    );

    let pm = create_package_manager(&fs, "/project");
    let resolve_from = Path::new("/project/.parcelrc");

    let deps = pm.read_additional_dev_deps("@atlaspack/transformer-compiled", resolve_from);
    assert_eq!(deps, Vec::<String>::new());
  }

  #[test]
  fn test_read_additional_dev_deps_ignores_non_string_entries() {
    let fs = Arc::new(InMemoryFileSystem::default());
    fs.write_file(
      &PathBuf::from("/project/package.json"),
      serde_json::json!({
        "atlaspack": {
          "@atlaspack/transformer-compiled": {
            "additionalDevDeps": ["@devtools/compiled-resolver", 42, null, "@custom/valid"]
          }
        }
      })
      .to_string(),
    );

    let pm = create_package_manager(&fs, "/project");
    let resolve_from = Path::new("/project/.parcelrc");

    let deps = pm.read_additional_dev_deps("@atlaspack/transformer-compiled", resolve_from);
    assert_eq!(
      deps,
      vec![
        "@devtools/compiled-resolver".to_string(),
        "@custom/valid".to_string(),
      ]
    );
  }

  #[test]
  fn test_read_additional_dev_deps_different_plugin_name() {
    let fs = Arc::new(InMemoryFileSystem::default());
    fs.write_file(
      &PathBuf::from("/project/package.json"),
      serde_json::json!({
        "atlaspack": {
          "@atlaspack/transformer-compiled": {
            "additionalDevDeps": ["@devtools/compiled-resolver"]
          },
          "@atlaspack/transformer-css": {
            "additionalDevDeps": ["@custom/css-plugin"]
          }
        }
      })
      .to_string(),
    );

    let pm = create_package_manager(&fs, "/project");
    let resolve_from = Path::new("/project/.parcelrc");

    let compiled_deps =
      pm.read_additional_dev_deps("@atlaspack/transformer-compiled", resolve_from);
    assert_eq!(
      compiled_deps,
      vec!["@devtools/compiled-resolver".to_string()]
    );

    let css_deps = pm.read_additional_dev_deps("@atlaspack/transformer-css", resolve_from);
    assert_eq!(css_deps, vec!["@custom/css-plugin".to_string()]);

    let unknown_deps = pm.read_additional_dev_deps("@atlaspack/transformer-unknown", resolve_from);
    assert_eq!(unknown_deps, Vec::<String>::new());
  }

  #[test]
  fn test_read_additional_dev_deps_handles_malformed_json_gracefully() {
    let fs = Arc::new(InMemoryFileSystem::default());
    fs.write_file(
      &PathBuf::from("/project/package.json"),
      "{ invalid json".to_string(),
    );

    let pm = create_package_manager(&fs, "/project");
    let resolve_from = Path::new("/project/.parcelrc");

    let deps = pm.read_additional_dev_deps("@atlaspack/transformer-compiled", resolve_from);
    assert_eq!(deps, Vec::<String>::new());
  }
}
