use std::path::PathBuf;

use atlaspack_core::config_loader::ConfigFile;
use atlaspack_core::config_loader::ConfigLoader;
use atlaspack_core::types::browsers::Browsers;
use atlaspack_core::types::engines::Engines;
use atlaspack_core::types::engines::EnginesBrowsers;
use atlaspack_core::types::AtlaspackOptions;
use atlaspack_core::types::Diagnostic;
use atlaspack_core::types::ErrorKind;
use atlaspack_filesystem::FileSystemRef;

use super::package_json::BrowsersList;
use super::package_json::PackageJson;

pub fn load_config(
  config_loader: &ConfigLoader,
  options: &AtlaspackOptions,
  file_system: &FileSystemRef,
) -> Result<ConfigFile<PackageJson>, anyhow::Error> {
  // TODO Invalidations
  let mut config = match config_loader.load_package_json::<PackageJson>() {
    Err(err) => {
      let diagnostic = err.downcast_ref::<Diagnostic>();

      if diagnostic.is_some_and(|d| d.kind != ErrorKind::NotFound) {
        return Err(err);
      }

      ConfigFile {
        contents: PackageJson::default(),
        path: PathBuf::default(),
        raw: String::default(),
      }
    }
    Ok(pkg) => pkg,
  };

  if let Some(engines) = config.contents.engines.as_ref() {
    if let Some(browsers) = &engines.browsers {
      if !Browsers::from(browsers).is_empty() {
        return Ok(config);
      }
    }
  }

  let env = options
    .env
    .as_ref()
    .and_then(|env| env.get("BROWSERSLIST_ENV").or_else(|| env.get("NODE_ENV")))
    .map(|e| e.to_owned())
    .unwrap_or_else(|| options.mode.to_string());

  let browsers = match config.contents.browserslist.clone() {
    None => {
      let browserslistrc_path = config_loader.project_root.join(".browserslistrc");

      // Loading .browserslistrc
      if file_system.is_file(browserslistrc_path.as_path()) {
        let browserslistrc = file_system.read_to_string(&browserslistrc_path)?;

        Some(EnginesBrowsers::from_browserslistrc(&browserslistrc)?)
      } else {
        None
      }
    }
    Some(browserslist) => Some(EnginesBrowsers::new(match browserslist {
      BrowsersList::Browser(browser) => vec![browser],
      BrowsersList::Browsers(browsers) => browsers,
      BrowsersList::BrowsersByEnv(browsers_by_env) => {
        browsers_by_env.get(&env).cloned().unwrap_or_default()
      }
    })),
  };

  if let Some(browserslist) = browsers {
    config.contents.engines = Some(Engines {
      browsers: Some(browserslist),
      ..config.contents.engines.unwrap_or_default()
    });
  }

  Ok(config)
}
