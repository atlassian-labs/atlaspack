use std::fs;

use super::install::InstallCommand;
use crate::context::Context;
use crate::platform::archive;
use crate::platform::exec::ExecOptions;
use crate::platform::exec::exec_blocking;
use crate::platform::fs_ext;
use crate::platform::http;
use crate::platform::package::GitPackage;
use crate::platform::path_ext::*;
use crate::platform::specifier::Specifier;
use crate::platform::temp_dir::TempDir;
use crate::public::json_serde::JsonSerde;
use crate::public::package_kind::PackageKind;
use crate::public::package_meta::PackageMeta;

pub fn install_from_git(
  ctx: Context,
  cmd: InstallCommand,
  version: &Specifier,
) -> anyhow::Result<()> {
  let pkg = GitPackage::from_name(&ctx.paths.versions_v1, version)?;
  let url = format!(
    "{}/{}.tar.gz",
    ctx.env.atlaspack_github_url,
    &version.version()
  );

  println!("Downloading");
  let bytes_archive = http::download_bytes(&url)?;

  println!("Extracting");
  let temp_dir = TempDir::new(&ctx.paths.temp)?;
  archive::tar_gz(bytes_archive.as_slice()).unpack(&temp_dir)?;
  let inner_temp = fs::read_dir(&temp_dir)?.try_first()?;

  let command_options = ExecOptions {
    cwd: Some(inner_temp.path()),
    ..Default::default()
  };

  if cmd.skip_postinstall {
    fs_ext::recreate_dir(&pkg.path)?;
    fs_ext::recreate_dir(pkg.contents())?;
    fs::rename(inner_temp.path(), pkg.contents())?;
    println!("Skipping build steps");
    return Ok(());
  }

  println!("Initializing");
  // Atlaspack needs a .git folder or the build will fail
  fs::create_dir_all(inner_temp.path().join(".git"))?;

  println!("Installing (yarn)");
  exec_blocking(["yarn", "install"], &command_options)?;

  println!("Building (Native)");
  exec_blocking(["yarn", "build-native-release"], &command_options)?;

  println!("Building (TypeScript)");
  exec_blocking(["yarn", "build"], &command_options)?;

  // Save a little space
  fs::remove_dir_all(inner_temp.path().join("target"))?;

  // Commit files to versions
  fs_ext::recreate_dir(&pkg.path)?;
  fs_ext::recreate_dir(pkg.contents())?;

  PackageMeta::write_to_file(
    &PackageMeta {
      kind: PackageKind::Git,
      version: Some(version.version().to_string()),
      specifier: Some(version.to_string()),
      checksum: None,
    },
    pkg.meta_file(),
  )?;

  fs::rename(inner_temp.path(), pkg.contents())?;

  Ok(())
}
