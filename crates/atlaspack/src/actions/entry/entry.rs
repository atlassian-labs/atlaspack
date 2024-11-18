use std::path::PathBuf;

use super::super::ActionQueue;
use super::super::ActionType;
use super::super::Compilation;
use super::super::TargetAction;
use crate::actions::Action;

#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct Entry {
  pub file_path: PathBuf,
  pub target: Option<String>,
}

#[derive(Hash, Debug)]
pub struct EntryAction {
  pub entry: String,
}

impl Action for EntryAction {
  async fn run(
    self,
    q: ActionQueue,
    Compilation {
      fs, project_root, ..
    }: &Compilation,
  ) -> anyhow::Result<()> {
    // TODO: Handle globs and directories
    let mut entry_path = PathBuf::from(self.entry.clone());
    if entry_path.is_relative() {
      entry_path = project_root.join(entry_path);
    };

    if !fs.is_file(&entry_path) {
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
