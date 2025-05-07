use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

use serde::Serialize;

use super::path_ext::find_ancestor_file;
use super::specifier::Specifier;
use crate::public::apvm_config::ApvmConfig;
use crate::public::json_serde::JsonSerde;

pub type ApvmRcRef = Rc<ApvmRc>;

#[derive(Clone, Debug, Default, Serialize)]
pub struct ApvmRc {
  pub exists: bool,
  pub path: PathBuf,
  versions: Rc<RefCell<HashMap<String, Specifier>>>,
  checksums: Rc<RefCell<HashMap<Specifier, String>>>,
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
  pub fn new(start_dir: &Path) -> anyhow::Result<Rc<Self>> {
    let results = find_ancestor_file(start_dir, "apvm.json")?;
    let Some(apvm_config_path) = results.first() else {
      return Ok(Rc::new(Self {
        exists: false,
        path: start_dir.join("apvm.json"),
        versions: Default::default(),
        checksums: Default::default(),
      }));
    };

    let Ok(apvm_config) = ApvmConfig::read_from_file(apvm_config_path) else {
      return Err(anyhow::anyhow!(
        "Invalid apvm.json at {:?}",
        apvm_config_path
      ));
    };

    let apvmrc = Rc::new(Self {
      exists: true,
      path: apvm_config_path.clone(),
      versions: Default::default(),
      checksums: Default::default(),
    });

    if let Some(versions) = &apvm_config.versions {
      for (alias, version) in versions {
        apvmrc
          .versions
          .borrow_mut()
          .insert(alias.clone(), Specifier::parse(version)?);
      }
    };

    if let Some(checksums) = &apvm_config.checksums {
      for (version, checksum) in checksums {
        apvmrc
          .checksums
          .borrow_mut()
          .insert(Specifier::parse(version)?, checksum.clone());
      }
    };

    Ok(apvmrc)
  }

  pub fn set_alias(&self, alias: impl AsRef<str>, version: Specifier) {
    self
      .versions
      .borrow_mut()
      .insert(alias.as_ref().to_string(), version);
  }

  pub fn get_alias(&self, alias: impl AsRef<str>) -> Option<Specifier> {
    self.versions.borrow().get(alias.as_ref()).cloned()
  }

  pub fn get_aliases(&self) -> Vec<(String, Specifier)> {
    self.versions.borrow().clone().into_iter().collect()
  }

  pub fn set_checksum(&self, specifier: &Specifier, checksum: impl AsRef<str>) {
    self
      .checksums
      .borrow_mut()
      .insert(specifier.clone(), checksum.as_ref().to_string());
  }

  pub fn get_checksum(&self, specifier: &Specifier) -> Option<String> {
    self.checksums.borrow().get(specifier).cloned()
  }

  pub fn tidy(&self) {
    let mut checksums = self.checksums.borrow_mut();
    let versions = self.versions.borrow();

    'block: for (specifier, _checksum) in checksums.clone().iter() {
      for (_, included) in versions.iter() {
        if included == specifier {
          continue 'block;
        }
      }
      checksums.remove(specifier);
    }
  }

  pub fn write_to_file(&self) -> anyhow::Result<()> {
    let mut versions = HashMap::<String, String>::new();
    let mut checksums = HashMap::<String, String>::new();

    for (version, specifier) in self.versions.borrow().iter() {
      versions.insert(version.clone(), specifier.to_string());
    }

    for (specifier, checksum) in self.checksums.borrow().iter() {
      checksums.insert(specifier.to_string(), checksum.clone());
    }

    ApvmConfig::write_to_file(
      &ApvmConfig {
        versions: Some(versions),
        checksums: Some(checksums),
      },
      &self.path,
    )
  }
}
