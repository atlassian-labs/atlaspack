use std::hash::Hash;
use std::path::PathBuf;

use anyhow::anyhow;
use async_trait::async_trait;

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
    // TODO: Handle globs and directories
    let mut entry_path = PathBuf::from(self.entry.clone());
    if entry_path.is_relative() {
      entry_path = request_context.project_root.join(entry_path);
    };

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

    Err(anyhow!("Unknown entry: {}", self.entry))
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

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
          target: None,
        }],
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
          file_path: entry_path,
          target: None,
        }],
      },
    );
  }
}
