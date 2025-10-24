use std::hash::Hash;
use std::path::PathBuf;

use async_trait::async_trait;
use atlaspack_core::config_loader::ConfigLoader;
use atlaspack_core::diagnostic_error;
use atlaspack_core::types::DiagnosticBuilder;
use serde::{Deserialize, Serialize};

use super::RequestResult;

use crate::request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError};

/// A resolved entry file for the build
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct Entry {
  pub file_path: PathBuf,
  pub package_path: PathBuf, // directory that contains the package.json file used to resolve dependencies etc.
  pub target: Option<String>,
}

/// Package.json target configuration
#[derive(Debug, Deserialize, Serialize)]
pub struct PackageTarget {
  pub source: Option<serde_json::Value>, // Can be string, array of strings, or null
}

/// Package.json structure for entry resolution
#[derive(Debug, Deserialize, Serialize)]
pub struct PackageJSON {
  pub source: Option<serde_json::Value>, // Can be string, array of strings, or null
  pub targets: Option<std::collections::HashMap<String, PackageTarget>>,
}

/// The EntryRequest resolves an entry path or glob to the actual file location
#[derive(Debug, Hash)]
pub struct EntryRequest {
  pub entry: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntryRequestOutput {
  pub entries: Vec<Entry>,
  pub files: Vec<PathBuf>, // Files that affect entry resolution (like package.json)
  pub globs: Vec<String>,  // Glob patterns that affect entry resolution
}

#[async_trait]
impl Request for EntryRequest {
  #[tracing::instrument(level = "info", skip_all)]
  async fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let mut entry_path = PathBuf::from(self.entry.clone());
    if entry_path.is_relative() {
      entry_path = request_context.project_root.join(entry_path);
    };

    let feature_flag_v3_handle_directory_entry_points = request_context
      .options
      .feature_flags
      .bool_enabled("v3HandleDirectoryEntryPoints");

    // Handle file entries
    if request_context.file_system().is_file(&entry_path) {
      let package_path = if entry_path.starts_with(&request_context.project_root) {
        request_context.project_root.clone()
      } else {
        entry_path
          .parent()
          .unwrap_or(&request_context.project_root)
          .to_path_buf()
      };

      return Ok(ResultAndInvalidations {
        result: RequestResult::Entry(EntryRequestOutput {
          entries: vec![Entry {
            file_path: entry_path,
            // Prior to v3HandleDirectoryEntryPoints, Entry did not have a package_path field.
            package_path: if feature_flag_v3_handle_directory_entry_points {
              package_path
            } else {
              PathBuf::new() // Empty for original behavior
            },
            target: None,
          }],
          files: vec![],
          globs: vec![],
        }),
        // TODO: invalidations
        invalidations: vec![],
      });
    }

    // Handle directory entries
    // only if v3HandleDirectoryEntryPoints feature flag is enabled
    if feature_flag_v3_handle_directory_entry_points
      && request_context.file_system().is_dir(&entry_path)
    {
      return self
        .handle_directory_entry(entry_path, request_context)
        .await;
    }

    Err(diagnostic_error!(
      DiagnosticBuilder::default().message(format!("Unknown entry: {}", self.entry))
    ))
  }
}

impl EntryRequest {
  async fn handle_directory_entry(
    &self,
    entry_path: PathBuf,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    // Create a ConfigLoader for this directory
    let config_loader = ConfigLoader {
      fs: request_context.file_system().clone(),
      project_root: request_context.project_root.clone(),
      search_path: entry_path.clone(),
    };

    // Use ConfigLoader to load package.json
    let package_json_file = config_loader.load_package_json::<PackageJSON>()?;

    let package_json = package_json_file.contents;
    let package_json_path = package_json_file.path;

    let mut entries = Vec::new();
    let files = vec![package_json_path];
    let globs = Vec::new();

    // Count targets with sources
    let mut targets_with_sources = 0;
    if let Some(targets) = &package_json.targets {
      for (target_name, target) in targets {
        if let Some(source) = &target.source {
          targets_with_sources += 1;
          let target_entries = self
            .resolve_target_sources(&entry_path, source, target_name, &request_context)
            .await?;
          entries.extend(target_entries);
        }
      }
    }

    // Check if all targets have sources
    let all_targets_have_source = targets_with_sources > 0
      && package_json.targets.is_some()
      && package_json.targets.as_ref().unwrap().len() == targets_with_sources;

    // If not all targets have sources, try package-level source
    if !all_targets_have_source && let Some(source) = &package_json.source {
      let package_entries = self
        .resolve_package_sources(&entry_path, source, &request_context)
        .await?;
      entries.extend(package_entries);
    }

    // Only return if we found valid entries
    if !entries.is_empty() {
      Ok(ResultAndInvalidations {
        result: RequestResult::Entry(EntryRequestOutput {
          entries,
          files,
          globs,
        }),
        invalidations: vec![],
      })
    } else {
      Err(diagnostic_error!(DiagnosticBuilder::default().message(
        format!("Could not find entry: {}", entry_path.display())
      )))
    }
  }

  async fn resolve_target_sources(
    &self,
    entry_path: &std::path::Path,
    source: &serde_json::Value,
    target_name: &str,
    request_context: &RunRequestContext,
  ) -> Result<Vec<Entry>, RunRequestError> {
    let sources = match source {
      serde_json::Value::String(s) => vec![s.clone()],
      serde_json::Value::Array(arr) => arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect(),
      _ => return Ok(vec![]),
    };

    let mut entries = Vec::new();
    for source in sources {
      // TODO: Handle globs in source
      let source_path = entry_path.join(&source);
      if request_context.file_system().is_file(&source_path) {
        entries.push(Entry {
          file_path: source_path,
          package_path: entry_path.to_path_buf(),
          target: Some(target_name.to_string()),
        });
      } else {
        // Match v2 behavior: throw error when source file doesn't exist
        return Err(diagnostic_error!(
          DiagnosticBuilder::default()
            .message(format!("{} does not exist.", source_path.display()))
        ));
      }
    }
    Ok(entries)
  }

  async fn resolve_package_sources(
    &self,
    entry_path: &std::path::Path,
    source: &serde_json::Value,
    request_context: &RunRequestContext,
  ) -> Result<Vec<Entry>, RunRequestError> {
    let sources = match source {
      serde_json::Value::String(s) => vec![s.clone()],
      serde_json::Value::Array(arr) => arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect(),
      _ => return Ok(vec![]),
    };

    let mut entries = Vec::new();
    for source in sources {
      // TODO: Handle globs in source
      let source_path = entry_path.join(&source);
      if request_context.file_system().is_file(&source_path) {
        entries.push(Entry {
          file_path: source_path,
          package_path: entry_path.to_path_buf(),
          target: None,
        });
      } else {
        // Match v2 behavior: throw error when source file doesn't exist
        return Err(diagnostic_error!(
          DiagnosticBuilder::default()
            .message(format!("{} does not exist.", source_path.display()))
        ));
      }
    }
    Ok(entries)
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use atlaspack_core::types::{AtlaspackOptions, FeatureFlags};
  use atlaspack_filesystem::FileSystem;
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;

  use crate::test_utils::{RequestTrackerTestOptions, request_tracker};

  use super::*;

  fn assert_entry_result(
    actual: Result<Arc<RequestResult>, anyhow::Error>,
    expected: EntryRequestOutput,
  ) {
    let Ok(result) = actual else {
      panic!("Request failed");
    };

    assert_eq!(result, Arc::new(RequestResult::Entry(expected)));
  }

  fn default_test_options() -> RequestTrackerTestOptions {
    let options = AtlaspackOptions {
      feature_flags: FeatureFlags::with_bool_flag("v3HandleDirectoryEntryPoints", true),
      ..AtlaspackOptions::default()
    };

    RequestTrackerTestOptions {
      atlaspack_options: options,
      ..RequestTrackerTestOptions::default()
    }
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_error_when_entry_is_not_found() {
    let request = EntryRequest {
      entry: String::from("src/a.js"),
    };

    let entry = request_tracker(RequestTrackerTestOptions::default())
      .run_request(request)
      .await;

    assert_eq!(
      entry.map_err(|e| e.to_string()),
      Err(String::from("Unknown entry: src/a.js"))
    )
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_file_entry_from_project_root() {
    let fs = Arc::new(InMemoryFileSystem::default());
    let project_root = PathBuf::from("atlaspack");
    let request = EntryRequest {
      entry: String::from("src/a.js"),
    };

    let entry_path = project_root.join("src").join("a.js");

    fs.write_file(&entry_path, String::default());

    let entry = request_tracker(RequestTrackerTestOptions {
      fs,
      project_root: project_root.clone(),
      ..default_test_options()
    })
    .run_request(request)
    .await;

    assert_entry_result(
      entry,
      EntryRequestOutput {
        entries: vec![Entry {
          file_path: entry_path,
          package_path: project_root, // With feature flag enabled
          target: None,
        }],
        files: vec![],
        globs: vec![],
      },
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_file_entry_from_project_root_feature_flag_off() {
    let fs = Arc::new(InMemoryFileSystem::default());
    let project_root = PathBuf::from("atlaspack");
    let request = EntryRequest {
      entry: String::from("src/a.js"),
    };

    let entry_path = project_root.join("src").join("a.js");

    fs.write_file(&entry_path, String::default());

    let entry = request_tracker(RequestTrackerTestOptions {
      fs,
      project_root: project_root.clone(),
      ..RequestTrackerTestOptions::default()
    })
    .run_request(request)
    .await;

    assert_entry_result(
      entry,
      EntryRequestOutput {
        entries: vec![Entry {
          file_path: entry_path,
          package_path: PathBuf::new(), // Empty when feature flag is disabled
          target: None,
        }],
        files: vec![],
        globs: vec![],
      },
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_file_entry_from_root() {
    let fs = Arc::new(InMemoryFileSystem::default());

    #[cfg(not(target_os = "windows"))]
    let root = PathBuf::from(std::path::MAIN_SEPARATOR_STR);

    #[cfg(target_os = "windows")]
    let root = PathBuf::from("c:\\windows");

    let entry_path = root.join("src").join("a.js");
    let request = EntryRequest {
      entry: root.join("src/a.js").to_string_lossy().into_owned(),
    };

    fs.write_file(&entry_path, String::default());

    let entry = request_tracker(RequestTrackerTestOptions {
      fs,
      project_root: PathBuf::from("atlaspack"),
      ..default_test_options()
    })
    .run_request(request)
    .await;

    assert_entry_result(
      entry,
      EntryRequestOutput {
        entries: vec![Entry {
          file_path: entry_path.clone(),
          package_path: entry_path.parent().unwrap().to_path_buf(), // With feature flag enabled
          target: None,
        }],
        files: vec![],
        globs: vec![],
      },
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_file_entry_from_root_feature_flag_off() {
    let fs = Arc::new(InMemoryFileSystem::default());

    #[cfg(not(target_os = "windows"))]
    let root = PathBuf::from(std::path::MAIN_SEPARATOR_STR);

    #[cfg(target_os = "windows")]
    let root = PathBuf::from("c:\\windows");

    let entry_path = root.join("src").join("a.js");
    let request = EntryRequest {
      entry: root.join("src/a.js").to_string_lossy().into_owned(),
    };

    fs.write_file(&entry_path, String::default());

    let entry = request_tracker(RequestTrackerTestOptions {
      fs,
      project_root: PathBuf::from("atlaspack"),
      ..RequestTrackerTestOptions::default()
    })
    .run_request(request)
    .await;

    assert_entry_result(
      entry,
      EntryRequestOutput {
        entries: vec![Entry {
          file_path: entry_path.clone(),
          package_path: PathBuf::new(), // Empty when feature flag is disabled
          target: None,
        }],
        files: vec![],
        globs: vec![],
      },
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_error_when_directory_has_no_package_json() {
    let fs = Arc::new(InMemoryFileSystem::default());
    let project_root = PathBuf::from("atlaspack");
    let request = EntryRequest {
      entry: String::from("src"),
    };

    let entry_path = project_root.join("src");
    fs.create_directory(&entry_path).unwrap();

    let entry = request_tracker(RequestTrackerTestOptions {
      fs,
      project_root: project_root.clone(),
      ..default_test_options()
    })
    .run_request(request)
    .await;

    assert_eq!(
      entry.map_err(|e| e.to_string()),
      Err(String::from(
        "Unable to locate package.json config file from atlaspack/src"
      ))
    )
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_entries_from_package_json_source() {
    let fs = Arc::new(InMemoryFileSystem::default());
    let project_root = PathBuf::from("atlaspack");
    let request = EntryRequest {
      entry: String::from("src"),
    };

    let entry_path = project_root.join("src");
    let package_json_path = entry_path.join("package.json");
    let source_file_path = entry_path.join("index.js");

    fs.create_directory(&entry_path).unwrap();
    fs.write_file(&source_file_path, String::default());
    fs.write_file(&package_json_path, r#"{"source": "index.js"}"#.to_string());

    let entry = request_tracker(RequestTrackerTestOptions {
      fs,
      project_root: project_root.clone(),
      ..default_test_options()
    })
    .run_request(request)
    .await;

    assert_entry_result(
      entry,
      EntryRequestOutput {
        entries: vec![Entry {
          file_path: source_file_path,
          package_path: entry_path,
          target: None,
        }],
        files: vec![package_json_path],
        globs: vec![],
      },
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_entries_from_package_json_targets() {
    let fs = Arc::new(InMemoryFileSystem::default());
    let project_root = PathBuf::from("atlaspack");
    let request = EntryRequest {
      entry: String::from("src"),
    };

    let entry_path = project_root.join("src");
    let package_json_path = entry_path.join("package.json");
    let source_file_path = entry_path.join("index.js");

    fs.create_directory(&entry_path).unwrap();
    fs.write_file(&source_file_path, String::default());
    fs.write_file(
      &package_json_path,
      r#"{"targets": {"main": {"source": "index.js"}}}"#.to_string(),
    );

    let entry = request_tracker(RequestTrackerTestOptions {
      fs,
      project_root: project_root.clone(),
      ..default_test_options()
    })
    .run_request(request)
    .await;

    assert_entry_result(
      entry,
      EntryRequestOutput {
        entries: vec![Entry {
          file_path: source_file_path,
          package_path: entry_path,
          target: Some("main".to_string()),
        }],
        files: vec![package_json_path],
        globs: vec![],
      },
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_error_when_directory_entry_feature_flag_off() {
    let fs = Arc::new(InMemoryFileSystem::default());
    let project_root = PathBuf::from("atlaspack");
    let request = EntryRequest {
      entry: String::from("src"),
    };

    let entry_path = project_root.join("src");
    fs.create_directory(&entry_path).unwrap();

    let entry = request_tracker(RequestTrackerTestOptions {
      fs,
      project_root: project_root.clone(),
      ..RequestTrackerTestOptions::default()
    })
    .run_request(request)
    .await;

    // When feature flag is disabled, directory entries should be treated as unknown
    assert_eq!(
      entry.map_err(|e| e.to_string()),
      Err(String::from("Unknown entry: src"))
    )
  }
}
