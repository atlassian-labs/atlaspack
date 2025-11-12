use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_filesystem::FileSystemRef;
use atlaspack_filesystem::search::find_ancestor_file;
use serde::de::DeserializeOwned;

use crate::{
  diagnostic_error,
  types::{CodeFrame, CodeHighlight, DiagnosticBuilder, DiagnosticError, ErrorKind, File},
};

pub type ConfigLoaderRef = Arc<ConfigLoader>;

/// Enables config to be loaded in various formats
#[derive(Debug)]
pub struct ConfigLoader {
  pub fs: FileSystemRef,
  pub project_root: PathBuf,
  pub search_path: PathBuf,
}

#[derive(Debug, PartialEq)]
pub struct ConfigFile<T> {
  pub contents: T,
  pub path: PathBuf,
  pub raw: String,
}

// TODO JavaScript configs, invalidations, dev deps, etc
impl ConfigLoader {
  pub fn load_json_config<Config: DeserializeOwned>(
    &self,
    filename: &str,
  ) -> Result<ConfigFile<Config>, DiagnosticError> {
    let path = find_ancestor_file(
      &*self.fs,
      &[filename],
      &self.search_path,
      &self.project_root,
    )
    .ok_or_else(|| {
      diagnostic_error!(
        DiagnosticBuilder::default()
          .kind(ErrorKind::NotFound)
          .message(format!(
            "Unable to locate {filename} config file from {}",
            self.search_path.display()
          ))
      )
    })?;

    let code = self.fs.read_to_string(&path)?;

    let contents = serde_json::from_str::<Config>(&code).map_err(|error| {
      diagnostic_error!(
        DiagnosticBuilder::default()
          .kind(ErrorKind::ParseError)
          .code_frames(vec![CodeFrame {
            code_highlights: vec![CodeHighlight::from([error.line(), error.column()])],
            ..CodeFrame::from(File {
              contents: code.clone(),
              path: path.clone()
            })
          }])
          .message(format!("Error parsing {}: {error}", path.display()))
      )
    })?;

    Ok(ConfigFile {
      contents,
      path,
      raw: code,
    })
  }

  pub fn load_package_json<Config: DeserializeOwned>(
    &self,
  ) -> Result<ConfigFile<Config>, anyhow::Error> {
    self.load_json_config::<Config>("package.json")
  }
}

#[cfg(test)]
mod tests {
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;

  use super::*;

  mod load_json_config {
    use std::sync::Arc;

    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct JsonConfig {}

    #[test]
    fn returns_an_error_when_the_config_does_not_exist() {
      let project_root = PathBuf::from("/project-root");
      let search_path = project_root.join("index");

      let config = ConfigLoader {
        fs: Arc::new(InMemoryFileSystem::default()),
        project_root,
        search_path: search_path.clone(),
      };

      assert_eq!(
        config
          .load_json_config::<JsonConfig>("config.json")
          .map_err(|err| err.to_string()),
        Err(format!(
          "Unable to locate config.json config file from {}",
          search_path.display()
        ))
      )
    }

    #[test]
    fn returns_an_error_when_the_config_is_outside_the_search_path() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = PathBuf::from("/project-root");
      let search_path = project_root.join("index");

      fs.write_file(
        &search_path.join("packages").join("config.json"),
        String::from("{}"),
      );

      let config = ConfigLoader {
        fs,
        project_root: PathBuf::default(),
        search_path: search_path.clone(),
      };

      assert_eq!(
        config
          .load_json_config::<JsonConfig>("config.json")
          .map_err(|err| err.to_string()),
        Err(format!(
          "Unable to locate config.json config file from {}",
          search_path.display()
        ))
      )
    }

    #[test]
    fn returns_an_error_when_the_config_is_outside_the_project_root() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = PathBuf::from("/project-root");
      let search_path = project_root.join("index");

      fs.write_file(&PathBuf::from("config.json"), String::from("{}"));

      let config = ConfigLoader {
        fs,
        project_root,
        search_path: search_path.clone(),
      };

      assert_eq!(
        config
          .load_json_config::<JsonConfig>("config.json")
          .map_err(|err| err.to_string()),
        Err(format!(
          "Unable to locate config.json config file from {}",
          search_path.display()
        ))
      )
    }

    #[test]
    fn returns_json_config_at_search_path() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = PathBuf::from("/project-root");
      let search_path = project_root.join("index");
      let config_path = search_path.join("config.json");

      fs.write_file(&config_path, String::from("{}"));

      let config = ConfigLoader {
        fs,
        project_root,
        search_path,
      };

      assert_eq!(
        config
          .load_json_config::<JsonConfig>("config.json")
          .map_err(|err| err.to_string()),
        Ok(ConfigFile {
          path: config_path,
          contents: JsonConfig {},
          raw: String::from("{}")
        })
      )
    }

    #[test]
    fn returns_json_config_at_project_root() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = PathBuf::from("/project-root");
      let search_path = project_root.join("index");
      let config_path = project_root.join("config.json");

      fs.write_file(&config_path, String::from("{}"));

      let config = ConfigLoader {
        fs,
        project_root,
        search_path,
      };

      assert_eq!(
        config
          .load_json_config::<JsonConfig>("config.json")
          .map_err(|err| err.to_string()),
        Ok(ConfigFile {
          path: config_path,
          contents: JsonConfig {},
          raw: String::from("{}")
        })
      )
    }
  }

  mod load_package_json_config {
    use std::sync::Arc;

    use super::*;

    fn package_json() -> String {
      String::from(
        r#"
        {
          "name": "atlaspack",
          "version": "1.0.0",
          "plugin": {
            "enabled": true
          }
        }
      "#,
      )
    }

    fn package_config() -> PackageJsonConfig {
      PackageJsonConfig {
        plugin: PluginConfig { enabled: true },
      }
    }

    #[derive(Debug, PartialEq, serde::Deserialize)]
    struct PluginConfig {
      enabled: bool,
    }

    #[derive(Debug, PartialEq, serde::Deserialize)]
    struct PackageJsonConfig {
      plugin: PluginConfig,
    }

    #[test]
    fn returns_an_error_when_package_json_does_not_exist() {
      let project_root = PathBuf::from("/project-root");
      let search_path = project_root.join("index");

      let config = ConfigLoader {
        fs: Arc::new(InMemoryFileSystem::default()),
        project_root,
        search_path: search_path.clone(),
      };

      assert_eq!(
        config
          .load_package_json::<PackageJsonConfig>()
          .map_err(|err| err.to_string()),
        Err(format!(
          "Unable to locate package.json config file from {}",
          search_path.display()
        ))
      )
    }

    #[test]
    fn returns_an_error_when_config_key_does_not_exist_at_search_path() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = PathBuf::from("/project-root");
      let search_path = project_root.join("index");
      let package_path = search_path.join("package.json");

      fs.write_file(&package_path, String::from("{}"));
      fs.write_file(&project_root.join("package.json"), package_json());

      let config = ConfigLoader {
        fs,
        project_root,
        search_path,
      };

      assert_eq!(
        config
          .load_package_json::<PackageJsonConfig>()
          .map_err(|err| err.to_string()),
        Err(format!(
          "Error parsing {}: missing field `plugin` at line 1 column 2",
          package_path.display()
        ))
      )
    }

    #[test]
    fn returns_an_error_when_config_key_does_not_exist_at_project_root() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = PathBuf::from("/project-root");
      let search_path = project_root.join("index");
      let package_path = project_root.join("package.json");

      fs.write_file(&package_path, String::from("{}"));

      let config = ConfigLoader {
        fs,
        project_root,
        search_path,
      };

      assert_eq!(
        config
          .load_package_json::<PackageJsonConfig>()
          .map_err(|err| err.to_string()),
        Err(format!(
          "Error parsing {}: missing field `plugin` at line 1 column 2",
          package_path.display()
        ))
      )
    }

    #[test]
    fn returns_package_config_at_search_path() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = PathBuf::from("/project-root");
      let search_path = project_root.join("index");
      let package_path = search_path.join("package.json");

      fs.write_file(&package_path, package_json());

      let config = ConfigLoader {
        fs,
        project_root,
        search_path,
      };

      assert_eq!(
        config
          .load_package_json::<PackageJsonConfig>()
          .map_err(|err| err.to_string()),
        Ok(ConfigFile {
          path: package_path,
          contents: package_config(),
          raw: package_json()
        })
      )
    }

    #[test]
    fn returns_package_config_at_project_root() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = PathBuf::from("/project-root");
      let search_path = project_root.join("index");
      let package_path = project_root.join("package.json");

      fs.write_file(&package_path, package_json());

      let config = ConfigLoader {
        fs,
        project_root,
        search_path,
      };

      assert_eq!(
        config
          .load_package_json::<PackageJsonConfig>()
          .map_err(|err| err.to_string()),
        Ok(ConfigFile {
          path: package_path,
          contents: package_config(),
          raw: package_json()
        })
      )
    }

    #[test]
    fn returns_an_error_when_package_json_is_malformed() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = PathBuf::from("/project-root");
      let search_path = project_root.join("index");
      let package_path = search_path.join("package.json");

      // Write malformed JSON
      fs.write_file(&package_path, String::from("{invalid json"));

      let config = ConfigLoader {
        fs,
        project_root,
        search_path,
      };

      assert_eq!(
        config
          .load_package_json::<PackageJsonConfig>()
          .map_err(|err| err.to_string()),
        Err(format!(
          "Error parsing {}: key must be a string at line 1 column 2",
          package_path.display()
        ))
      )
    }

    #[test]
    fn returns_an_error_when_package_json_has_trailing_comma() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = PathBuf::from("/project-root");
      let search_path = project_root.join("index");
      let package_path = search_path.join("package.json");

      // Write JSON with trailing comma
      fs.write_file(&package_path, String::from(r#"{"name": "test",}"#));

      let config = ConfigLoader {
        fs,
        project_root,
        search_path,
      };

      assert_eq!(
        config
          .load_package_json::<PackageJsonConfig>()
          .map_err(|err| err.to_string()),
        Err(format!(
          "Error parsing {}: trailing comma at line 1 column 17",
          package_path.display()
        ))
      )
    }

    #[test]
    fn returns_an_error_when_package_json_has_unclosed_brace() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = PathBuf::from("/project-root");
      let search_path = project_root.join("index");
      let package_path = search_path.join("package.json");

      // Write JSON with unclosed brace
      fs.write_file(&package_path, String::from(r#"{"name": "test""#));

      let config = ConfigLoader {
        fs,
        project_root,
        search_path,
      };

      assert_eq!(
        config
          .load_package_json::<PackageJsonConfig>()
          .map_err(|err| err.to_string()),
        Err(format!(
          "Error parsing {}: EOF while parsing an object at line 1 column 15",
          package_path.display()
        ))
      )
    }

    #[test]
    fn returns_an_error_when_package_json_is_empty() {
      let fs = Arc::new(InMemoryFileSystem::default());
      let project_root = PathBuf::from("/project-root");
      let search_path = project_root.join("index");
      let package_path = search_path.join("package.json");

      // Write empty file
      fs.write_file(&package_path, String::from(""));

      let config = ConfigLoader {
        fs,
        project_root,
        search_path,
      };

      assert_eq!(
        config
          .load_package_json::<PackageJsonConfig>()
          .map_err(|err| err.to_string()),
        Err(format!(
          "Error parsing {}: EOF while parsing a value at line 1 column 0",
          package_path.display()
        ))
      )
    }
  }
}
