use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;

use super::path_ext::find_ancestor_file;
use super::specifier::Specifier;
use crate::public::apvm_config::ApvmConfig;
use crate::public::json_serde::JsonSerde;

#[derive(Clone, Debug, Default, Serialize)]
pub struct ApvmRc {
  pub path: PathBuf,
  pub versions: HashMap<String, Specifier>,
  pub checksums: HashMap<Specifier, String>,
}

impl ApvmRc {
  /// This will traverse up the directory tree, looking for "package.json"
  /// and extracting the "package.json#atlaspack.version" keys.
  ///
  /// Example:
  ///
  /// ```json
  /// {
  ///   "version": "2.10.0",
  ///   "versions": {
  ///     // If default is specified here,
  ///     // the top level one is ignored
  ///     "default": "2.10.0",
  ///     "next": "2.11.0",
  ///     "arbitrary": "2.12.0"
  ///   },
  ///   "checksums": {
  ///     "2.10.0": "47cbce53a2..."
  ///     "2.11.0": "10a9e8e542..."
  ///     "2.12.0": "e2d94feb6b..."
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
    let results = find_ancestor_file(start_dir, "apvm.json")?;
    let Some(apvm_config_path) = results.first() else {
      return Ok(None);
    };
    let Ok(apvm_config) = ApvmConfig::read_from_file(apvm_config_path) else {
      return Err(anyhow::anyhow!(
        "Invalid apvm.json at {:?}",
        apvm_config_path
      ));
    };

    let mut apvmrc = Self {
      path: apvm_config_path.clone(),
      versions: HashMap::new(),
      checksums: HashMap::new(),
    };

    if let Some(version) = &apvm_config.version {
      apvmrc
        .versions
        .insert("default".to_string(), Specifier::parse(version)?);
    };

    if let Some(versions) = &apvm_config.versions {
      for (alias, version) in versions {
        apvmrc
          .versions
          .insert(alias.clone(), Specifier::parse(version)?);
      }
    };

    if let Some(checksums) = &apvm_config.checksums {
      for (version, checksum) in checksums {
        apvmrc
          .checksums
          .insert(Specifier::parse(version)?, checksum.clone());
      }
    };

    Ok(Some(apvmrc))
  }
}
