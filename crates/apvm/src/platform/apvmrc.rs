use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use super::origin::VersionTarget;
use super::package_json::PackageJson;
use super::path_ext::find_ancestor_file;

#[allow(unused)]
#[derive(Clone, Debug, Default)]
pub struct ApvmRc {
  pub path: PathBuf,
  pub version_target: Option<VersionTarget>,
  pub version_target_aliases: HashMap<String, VersionTarget>,
}

impl ApvmRc {
  /// This will traverse up the directory tree, looking for "package.json"
  /// and extracting the "package.json#atlaspack.version" keys.
  ///
  /// Example:
  ///
  /// ```json5
  /// {
  ///   "name": "some-package-json",
  ///   "atlaspack": {
  ///     "version": "2.10.0",
  ///     "versions": {
  ///       "default": "2.10.0",    // If default is specified here, the top level one is ignored
  ///       "next": "2.11.0",
  ///       "arbitrary": "2.12.0"
  ///     }
  ///   }
  /// }
  /// ```
  ///
  /// Where the CLI usage is:
  ///
  /// ```bash
  /// # Will install "package.json#atlaspack.version" by default
  /// npm install @atlaspack/apvm                 # Installs 2.10.0
  ///
  /// # Install and link a version by alias
  /// apvm npm link --install --alias next        # Installs 2.11.0
  /// apvm npm link -ia arbitrary                 # Installs 2.12.0
  ///
  /// # Install and link a version by specifier
  /// apvm npm link --install 2.13.0              # Installs 2.13.0
  /// ```
  pub fn detect(start_dir: &Path) -> anyhow::Result<Option<Self>> {
    for package_json_path in find_ancestor_file(start_dir, "package.json")? {
      let Ok(package_json) = PackageJson::parse_from_file(&package_json_path) else {
        continue;
      };

      let Some(atlaspack) = package_json.atlaspack else {
        continue;
      };

      let mut apvmrc = Self {
        path: package_json_path,
        version_target: None,
        version_target_aliases: HashMap::new(),
      };

      if let Some(version) = atlaspack.version {
        apvmrc.version_target = Some(VersionTarget::parse(version)?);
      };

      if let Some(versions) = atlaspack.versions {
        for (alias, specifier) in versions {
          if alias == "default" {
            apvmrc.version_target = Some(VersionTarget::parse(specifier)?);
            continue;
          }
          apvmrc
            .version_target_aliases
            .insert(alias, VersionTarget::parse(specifier)?);
        }
      };

      return Ok(Some(apvmrc));
    }

    Ok(None)
  }
}
