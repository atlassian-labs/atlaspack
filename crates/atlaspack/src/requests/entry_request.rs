use std::hash::Hash;
use std::path::PathBuf;

use anyhow::anyhow;
use async_trait::async_trait;

use super::target_request::package_json::PackageJson;
use super::RequestResult;

use crate::request_tracker::{Request, ResultAndInvalidations, RunRequestContext, RunRequestError};

/// A resolved entry file for the build
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct Entry {
  pub file_path: PathBuf,
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
}

#[async_trait]
impl Request for EntryRequest {
  #[tracing::instrument(level = "info", skip_all)]
  async fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    println!("EntryRequest: {:?}", self.entry);
    println!(
      "request_context.project_root: {:?}",
      request_context.project_root
    );
    // println!("request_context.options: {:?}", request_context.options);

    // TODO: Handle globs
    let mut entry_path = PathBuf::from(self.entry.clone());
    if entry_path.is_relative() {
      entry_path = request_context.project_root.join(entry_path);
    };

    println!("before file check");

    if request_context.file_system().is_file(&entry_path) {
      return Ok(ResultAndInvalidations {
        result: RequestResult::Entry(EntryRequestOutput {
          entries: vec![Entry {
            file_path: entry_path,
            target: None,
          }],
        }),
        // TODO: invalidations
        invalidations: vec![],
      });
    }

    println!("after file check");

    // Handle directories by reading package.json targets configuration
    if request_context.file_system().is_dir(&entry_path) {
      println!("EntryRequest: Processing directory: {:?}", entry_path);
      let package_json_path = entry_path.join("package.json");
      println!("EntryRequest: Looking for package.json at: {:?}", package_json_path);
      if request_context.file_system().is_file(&package_json_path) {
        println!("EntryRequest: Found package.json file");
        let package_json_content = request_context
          .file_system()
          .read_to_string(&package_json_path)?;
        println!("EntryRequest: Package.json content length: {}", package_json_content.len());
        println!("EntryRequest: Package.json content preview: {}", &package_json_content[..std::cmp::min(500, package_json_content.len())]);
        let package_json: PackageJson = serde_json::from_str(&package_json_content)?;
        println!("EntryRequest: Successfully parsed package.json");

        let mut entries = Vec::new();

        // Get the appropriate target based on the build mode
        let target_name = match request_context.options.mode {
          atlaspack_core::types::BuildMode::Development => "development",
          atlaspack_core::types::BuildMode::Production => "production",
          _ => "production", // Default to production
        };

        println!("EntryRequest: Looking for target: {}", target_name);
        println!("EntryRequest: Available custom targets: {:?}", package_json.targets.custom_targets.keys().collect::<Vec<_>>());

        // Check custom targets first
        if let Some(target) = package_json.targets.custom_targets.get(target_name) {
          println!("EntryRequest: Found target: {}", target_name);
          if let Some(source) = &target.source {
            println!("EntryRequest: Target has source: {:?}", source);
            match source {
              atlaspack_core::types::SourceField::Source(source_file) => {
                println!("EntryRequest: Processing single source file: {}", source_file);
                let source_path = entry_path.join(source_file);
                println!("EntryRequest: Full source path: {:?}", source_path);
                if request_context.file_system().is_file(&source_path) {
                  println!("EntryRequest: Source file exists");
                  entries.push(Entry {
                    file_path: source_path,
                    target: Some(target_name.to_string()),
                  });
                } else {
                  println!("EntryRequest: Source file does not exist");
                }
              }
              atlaspack_core::types::SourceField::Sources(sources) => {
                println!("EntryRequest: Processing multiple source files: {:?}", sources);
                for source_file in sources {
                  let source_path = entry_path.join(source_file);
                  println!("EntryRequest: Full source path: {:?}", source_path);
                  if request_context.file_system().is_file(&source_path) {
                    println!("EntryRequest: Source file exists");
                    entries.push(Entry {
                      file_path: source_path,
                      target: Some(target_name.to_string()),
                    });
                  } else {
                    println!("EntryRequest: Source file does not exist");
                  }
                }
              }
            }
          } else {
            println!("EntryRequest: Target has no source field");
          }
        } else {
          println!("EntryRequest: Target not found: {}", target_name);
        }

        if !entries.is_empty() {
          return Ok(ResultAndInvalidations {
            result: RequestResult::Entry(EntryRequestOutput { entries }),
            // TODO: invalidations
            invalidations: vec![],
          });
        }
      }
    }

    Err(anyhow!("Unknown entry: {}", self.entry))
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;

  use crate::test_utils::{request_tracker, RequestTrackerTestOptions};

  use super::*;

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

    assert_eq!(
      entry.map_err(|e| e.to_string()),
      Ok(RequestResult::Entry(EntryRequestOutput {
        entries: vec![Entry {
          file_path: entry_path,
          target: None,
        }]
      }))
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

    assert_eq!(
      entry.map_err(|e| e.to_string()),
      Ok(RequestResult::Entry(EntryRequestOutput {
        entries: vec![Entry {
          file_path: entry_path,
          target: None,
        }]
      }))
    );
  }
}
