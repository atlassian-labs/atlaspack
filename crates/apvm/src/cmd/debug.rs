use std::fs;

// Hidden utilities to access dynamically set values
use clap::Parser;
use clap::Subcommand;

use crate::context::Context;
use crate::platform::origin::VersionTarget;
use crate::platform::path_ext::PathExt;

#[derive(Debug, Subcommand, Clone)]
pub enum DebugCommandType {
  /// Path to the current package in APVM_DIR
  Path,
  /// Node.js resolution for the path to a package by name
  Resolve {
    /// Package to resolve, leave blank for "atlaspack"
    specifier: Option<String>,
    /// Get package path to node_modules
    #[arg(short = 'l', long = "link")]
    link: bool,
  },
}

#[derive(Debug, Parser)]
pub struct DebugCommand {
  #[clap(subcommand)]
  pub query: Option<DebugCommandType>,
}

#[rustfmt::skip]
pub fn main(ctx: Context, cmd: DebugCommand) -> anyhow::Result<()> {
  match cmd.query {
    None => {
      dbg!(&ctx);
    },
    Some(DebugCommandType::Path) => {
      let Some(active) = ctx.active_version else {
        return Err(anyhow::anyhow!("No version selected"));
      };
      print!("{}", active.package.path.try_to_string()?);
    },
    Some(DebugCommandType::Resolve{specifier, link}) => {
      let Some(active) = ctx.active_version else {
        return Err(anyhow::anyhow!("No version selected"));
      };

      if !matches!(active.package.version_target, VersionTarget::Npm(_)) {
        return Err(anyhow::anyhow!("Can only resolve npm package type"));
      }

      let package_path = {
        if link {
          let Some(node_modules_path) = active.node_modules_path  else {
            return Err(anyhow::anyhow!("Not linked into node_modules"));  
          };
          node_modules_path
        } else {
          active.package.path
        }
      };

      let Some(specifier) = specifier else {
        print!("{}", package_path.try_to_string()?);
        return Ok(());
      };

      match specifier.as_str() {
        "atlaspack" => print!("{}", package_path.try_to_string()?),
        name => {
          let Some(name_stripped) = name.strip_prefix("@atlaspack/") else {
            return Err(anyhow::anyhow!("Invalid specifier"));
          };

          for entry in fs::read_dir(package_path.join("lib"))? {
            let entry = entry?;
            let entry_path = entry.path();

            if fs::metadata(&entry_path)?.is_dir() {
              continue;
            }

            let file_stem = entry_path.try_file_stem()?;
            if file_stem.starts_with("vendor.") {
              continue;
            }

            if name_stripped == file_stem {
              print!("{}", entry_path.try_to_string()?);
              break;
            } 
          }

        }
      }
    },
  }

  Ok(())
}
