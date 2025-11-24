use std::hash::Hash;
use std::path::{Path, PathBuf};

use crate::requests::target_request::package_json::{BuiltInTargetDescriptor, PackageJson};
use async_trait::async_trait;
use atlaspack_core::config_loader::ConfigLoader;
use atlaspack_core::diagnostic_error;
use atlaspack_core::types::{DiagnosticBuilder, SourceField};

use super::RequestResult;

use crate::request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError};

/// A resolved entry file for the build
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct Entry {
  pub file_path: PathBuf,
  pub package_path: PathBuf, // directory that contains the package.json file used to resolve dependencies etc.
  pub target: Option<String>,
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

    // Handle file entries
    if request_context.file_system().is_file(&entry_path) {
      let result = self.handle_file_entry(entry_path, &request_context)?;
      tracing::debug!("EntryRequestOutput (file): {:#?}", result);
      return Ok(result);
    }

    // Handle directory entries
    if request_context.file_system().is_dir(&entry_path) {
      let result = self.handle_directory_entry(entry_path, request_context)?;
      tracing::debug!("EntryRequestOutput (directory): {:#?}", result);
      return Ok(result);
    }

    Err(diagnostic_error!(
      DiagnosticBuilder::default().message(format!("Unknown entry: {}", self.entry))
    ))
  }
}

impl EntryRequest {
  /// Handles a file entry by determining its package path and creating an Entry
  fn handle_file_entry(
    &self,
    entry_path: PathBuf,
    request_context: &RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let package_path = if entry_path.starts_with(&request_context.project_root) {
      request_context.project_root.clone()
    } else {
      entry_path
        .parent()
        .unwrap_or(&request_context.project_root)
        .to_path_buf()
    };

    Ok(ResultAndInvalidations {
      result: RequestResult::Entry(EntryRequestOutput {
        entries: vec![Entry {
          file_path: entry_path,
          package_path,
          target: None,
        }],
        files: vec![],
        globs: vec![],
      }),
      // TODO: invalidations
      invalidations: vec![],
    })
  }

  fn handle_directory_entry(
    &self,
    entry_path: PathBuf,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    // When provided with a directory as an entry point, load the package.json file from the given directory.
    let config_loader = ConfigLoader {
      fs: request_context.file_system().clone(),
      project_root: request_context.project_root.clone(),
      search_path: entry_path.clone(),
    };

    let package_json_file = config_loader.load_package_json::<PackageJson>()?;

    let package_json = package_json_file.contents;

    let package_json_path = package_json_file.path;

    let mut entries = Vec::new();
    let files = vec![package_json_path];
    let globs = Vec::new();

    // Process target-specific sources first
    // Targets take precedence over package-level source when they define their own source.
    let mut targets_with_sources = 0;
    let mut enabled_targets_count = 0;

    // Process built-in targets (browser, main, module, types)
    let builtin_targets = [
      ("browser", &package_json.targets.browser),
      ("main", &package_json.targets.main),
      ("module", &package_json.targets.module),
      ("types", &package_json.targets.types),
    ];

    for (target_name, builtin_target) in &builtin_targets {
      if let Some(target) = builtin_target {
        match target {
          BuiltInTargetDescriptor::Disabled(_) => {
            // Skip disabled targets (e.g., "main": false)
            continue;
          }
          BuiltInTargetDescriptor::TargetDescriptor(target_descriptor) => {
            enabled_targets_count += 1;
            if let Some(source) = &target_descriptor.source {
              targets_with_sources += 1;
              let target_entries =
                self.resolve_sources(&entry_path, source, Some(target_name), &request_context)?;
              entries.extend(target_entries);
            }
          }
        }
      }
    }

    // Process custom targets
    for (target_name, target_descriptor) in &package_json.targets.custom_targets {
      enabled_targets_count += 1;
      if let Some(source) = &target_descriptor.source {
        targets_with_sources += 1;
        let target_entries =
          self.resolve_sources(&entry_path, source, Some(target_name), &request_context)?;
        entries.extend(target_entries);
      }
    }

    // Determine if we should use package-level source as fallback
    //
    // Package-level source is used when:
    // 1. No targets are defined, OR
    // 2. Some targets exist but don't have their own source defined
    //
    // Example scenario:
    //   {
    //     "source": "fallback.js",  ← Used as fallback for targets without source
    //     "targets": {
    //       "main": false,  ← Disabled, ignored
    //       "development": { "source": "index.js" },  ← Has its own source
    //       "alt": {}  ← No source, will use "fallback.js"
    //     }
    //   }
    let all_targets_have_source =
      targets_with_sources > 0 && enabled_targets_count == targets_with_sources;

    // Get package-level source from the flattened fields
    if !all_targets_have_source && let Some(source_value) = package_json.fields.get("source") {
      // Convert JSON value to SourceField
      if let Ok(source_field) = serde_json::from_value::<SourceField>(source_value.clone()) {
        let package_entries =
          self.resolve_sources(&entry_path, &source_field, None, &request_context)?;
        entries.extend(package_entries);
      }
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

  /// Resolves source files from a SourceField into Entry objects.
  ///
  /// This method handles both target-specific sources and package-level sources.
  /// - If target_name is Some, the entries will be associated with that target
  /// - If target_name is None, the entries are package-level (no specific target)
  fn resolve_sources(
    &self,
    entry_path: &Path,
    source: &SourceField,
    target_name: Option<&str>,
    request_context: &RunRequestContext,
  ) -> Result<Vec<Entry>, RunRequestError> {
    let sources = match source {
      SourceField::Source(s) => vec![s.clone()],
      SourceField::Sources(arr) => arr.clone(),
    };

    let mut entries = Vec::new();
    for source in sources {
      // TODO: Handle globs in source
      let source_path = entry_path.join(&source);
      if request_context.file_system().is_file(&source_path) {
        entries.push(Entry {
          file_path: source_path,
          package_path: entry_path.to_path_buf(),
          target: target_name.map(|s| s.to_string()),
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
      ..RequestTrackerTestOptions::default()
    })
    .run_request(request)
    .await;

    assert_entry_result(
      entry,
      EntryRequestOutput {
        entries: vec![Entry {
          file_path: entry_path,
          package_path: project_root,
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
      ..RequestTrackerTestOptions::default()
    })
    .run_request(request)
    .await;

    assert_entry_result(
      entry,
      EntryRequestOutput {
        entries: vec![Entry {
          file_path: entry_path.clone(),
          package_path: entry_path.parent().unwrap().to_path_buf(),
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
      ..RequestTrackerTestOptions::default()
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
      ..RequestTrackerTestOptions::default()
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
      ..RequestTrackerTestOptions::default()
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
  async fn returns_entries_from_targets_with_package_source_fallback() {
    let fs = Arc::new(InMemoryFileSystem::default());
    let project_root = PathBuf::from("atlaspack");
    let request = EntryRequest {
      entry: String::from("src"),
    };

    let entry_path = project_root.join("src");
    let package_json_path = entry_path.join("package.json");
    let main_file = entry_path.join("main.js");
    let fallback_file = entry_path.join("fallback.js");

    fs.create_directory(&entry_path).unwrap();
    fs.write_file(&main_file, String::default());
    fs.write_file(&fallback_file, String::default());
    fs.write_file(
      &package_json_path,
      r#"{
        "source": "fallback.js",
        "targets": {
          "main": {"source": "main.js"},
          "alt": {}
        }
      }"#
        .to_string(),
    );

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
        entries: vec![
          Entry {
            file_path: main_file,
            package_path: entry_path.clone(),
            target: Some("main".to_string()),
          },
          Entry {
            file_path: fallback_file,
            package_path: entry_path.clone(),
            target: None, // From package-level source, not associated with a specific target
          },
        ],
        files: vec![package_json_path],
        globs: vec![],
      },
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_entries_ignoring_disabled_targets() {
    let fs = Arc::new(InMemoryFileSystem::default());
    let project_root = PathBuf::from("atlaspack");
    let request = EntryRequest {
      entry: String::from("src"),
    };

    let entry_path = project_root.join("src");
    let package_json_path = entry_path.join("package.json");
    let dev_file = entry_path.join("dev.js");
    let prod_file = entry_path.join("prod.js");

    fs.create_directory(&entry_path).unwrap();
    fs.write_file(&dev_file, String::default());
    fs.write_file(&prod_file, String::default());
    fs.write_file(
      &package_json_path,
      r#"{
        "targets": {
          "main": false,
          "development": {"source": "dev.js"},
          "production": {"source": "prod.js"}
        }
      }"#
        .to_string(),
    );

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
        entries: vec![
          Entry {
            file_path: dev_file,
            package_path: entry_path.clone(),
            target: Some("development".to_string()),
          },
          Entry {
            file_path: prod_file,
            package_path: entry_path.clone(),
            target: Some("production".to_string()),
          },
        ],
        files: vec![package_json_path],
        globs: vec![],
      },
    );
  }
}
