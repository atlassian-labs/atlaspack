use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;

use super::path_ext::find_ancestor_file;
use super::specifier::Specifier;
use crate::public::json_serde::JsonSerde;
use crate::public::package_json::PackageJson;

#[derive(Clone, Debug, Default, Serialize)]
pub struct ApvmRc {
  pub path: PathBuf,
  pub version_aliases: HashMap<String, Specifier>,
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
  pub fn detect(start_dir: &Path) -> anyhow::Result<Self> {
    let mut apvmrc = Self {
      path: Default::default(),
      version_aliases: HashMap::new(),
    };

    for package_json_path in find_ancestor_file(start_dir, "package.json")? {
      let Ok(package_json) = PackageJson::read_from_file(&package_json_path) else {
        continue;
      };

      let Some(atlaspack) = package_json.atlaspack else {
        continue;
      };

      if let Some(specifier) = atlaspack.version {
        apvmrc
          .version_aliases
          .insert("default".to_string(), Specifier::parse(specifier)?);
      };

      if let Some(versions) = atlaspack.versions {
        for (alias, specifier) in versions {
          apvmrc
            .version_aliases
            .insert(alias, Specifier::parse(specifier)?);
        }
      };

      apvmrc.path = package_json_path;
      break;
    }

    Ok(apvmrc)
  }
}
