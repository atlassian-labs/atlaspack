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

  #[tracing::instrument(level = "info", skip(self, resolve_from), ret)]
  fn resolve_dev_dependency(
    &self,
    package_name: &str,
    resolve_from: &std::path::Path,
  ) -> anyhow::Result<DevDep> {
    let crate::Resolution { resolved } = self.resolve(package_name, resolve_from)?;

    match atlaspack_dev_dep_resolver::build_esm_graph(
      &resolved,
      &self.project_root,
      &self.resolver.cache,
      &self.dev_dep_cache,
    ) {
      Err(err) => {
        // For now we'll ignore dev dev resolution errors and just treat it as
        // uncachable
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
      Ok(result) => {
        Ok(DevDep {
          file_path: resolved,
          // For we're just mapping to an option and ignoring errors
          // If we want more advanced reporting on cachability we can change this
          // later
          hash: self.hash_invalidations(&result).ok(),
        })
      }
    }
  }
}

impl NodePackageManager<'_> {
  #[tracing::instrument(level = "debug", skip_all)]
  fn hash_invalidations(&self, invalidations: &Invalidations) -> anyhow::Result<u64> {
    if invalidations
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
          return Err(anyhow!(
            "Dev dependency is not cacheable because it invalidates on file create (glob): {:?}",
            glob
          ));
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
