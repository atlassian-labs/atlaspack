use std::path::PathBuf;
use std::sync::Arc;

use super::super::ActionQueue;
use super::super::ActionType;
use super::super::TargetAction;
use crate::compilation::Compilation;

#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct Entry {
  pub file_path: PathBuf,
  pub target: Option<String>,
}

#[derive(Debug)]
pub struct EntryAction {
  pub entry: String,
}

impl EntryAction {
  pub async fn run(
    self,
    c: Arc<Compilation>,
    q: ActionQueue,
  ) -> anyhow::Result<()> {
    // TODO: Handle globs and directories
    let mut entry_path = PathBuf::from(self.entry.clone());
    if entry_path.is_relative() {
      entry_path = c.project_root.join(entry_path);
    };

    if !c.fs.is_file(&entry_path) {
      return Ok(());
    }

    q.next(ActionType::Target(TargetAction {
      entry: Entry {
        file_path: entry_path,
        target: None,
      },
    }))
  }
}
