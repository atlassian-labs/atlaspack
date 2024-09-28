use atlaspack_filesystem::vcs_integration::{get_changed_files, FailureMode, VCSState};
use clap::Parser;

#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
struct Options {
  #[command(subcommand)]
  command: Command,
}

#[derive(Debug, Clone, clap::Subcommand)]
enum Command {
  Snapshot {
    /// Repository root to open
    #[arg(short, long)]
    repository_root: std::path::PathBuf,
    /// Patterns to exclude dirty files from the snapshot
    #[arg(short, long)]
    exclude: Vec<String>,
  },
  Diff {
    /// Repository root to open
    #[arg(short, long)]
    repository_root: std::path::PathBuf,
    /// Start revision
    #[arg(short, long)]
    start_rev: String,
    /// Optional end revision, defaults to HEAD
    #[arg(short, long)]
    end_rev: Option<String>,
  },
}

fn main() {
  tracing_subscriber::fmt::init();

  let Options { command } = Options::parse();

  match command {
    Command::Snapshot {
      repository_root,
      exclude,
    } => {
      let vcs_state = VCSState::read_from_repository(
        &repository_root,
        &exclude,
        FailureMode::IgnoreMissingNodeModules,
      )
      .unwrap();
      println!("{}", serde_json::to_string(&vcs_state).unwrap());
    }
    Command::Diff {
      repository_root,
      start_rev,
      end_rev,
    } => {
      let changes = get_changed_files(
        &repository_root,
        &start_rev,
        end_rev.as_deref().unwrap_or("HEAD"),
        FailureMode::IgnoreMissingNodeModules,
      )
      .unwrap();

      for change in changes {
        let change = change.strip_prefix(&repository_root).unwrap();
        println!("{}", change.display());
      }
    }
  }
}
