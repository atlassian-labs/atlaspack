use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;

use crate::platform::path_ext::*;

#[derive(Debug, Default, Clone)]
pub struct ExecOptions {
  pub cwd: Option<PathBuf>,
  pub stdio: bool,
  pub env: Option<HashMap<String, String>>,
}

pub fn exec_blocking<I, S>(args: I, options: &ExecOptions) -> anyhow::Result<()>
where
  I: IntoIterator<Item = S>,
  S: AsRef<OsStr>,
{
  let mut args = args
    .into_iter()
    .map(|v| v.as_ref().try_to_string().expect("Unable to parse args"))
    .collect::<Vec<String>>();
  let arg0 = args.remove(0);

  let mut command = std::process::Command::new(arg0);

  command.args(args);

  if let Some(cwd) = &options.cwd {
    command.current_dir(cwd);
  }

  if let Some(extra_env) = &options.env {
    for (key, val) in extra_env {
      command.env(key, val);
    }
  }

  let mut stdio = options.stdio;
  if log::log_enabled!(target: "Global", log::Level::Debug) {
    stdio = true
  }

  if !stdio {
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());
  } else {
    command.stdout(std::process::Stdio::inherit());
    command.stderr(std::process::Stdio::inherit());
  }

  let status = command.status()?;

  if !status.success() {
    return Err(anyhow::anyhow!("Process exited with status {}", status));
  }

  Ok(())
}
