use std::fs;

use serde::Deserialize;

use super::install::InstallCommand;
use crate::context::Context;
use crate::platform::archive;
use crate::platform::constants as c;
use crate::platform::exec::ExecOptions;
use crate::platform::exec::exec_blocking;
use crate::platform::fs_ext;
use crate::platform::hash::Integrity;
use crate::platform::http;
use crate::platform::package::NpmPackage;
use crate::platform::path_ext::*;
use crate::platform::runtime::resolve_executable;
use crate::platform::specifier::Specifier;
use crate::public::json_serde::JsonSerde;
use crate::public::package_kind::PackageKind;
use crate::public::package_meta::PackageMeta;

#[derive(Debug, Deserialize)]
struct NpmApiResponse {
  version: String,
  dist: NpmApiResponseDist,
}

#[derive(Debug, Deserialize)]
struct NpmApiResponseDist {
  tarball: String,
  integrity: String,
}

pub fn resolve_from_npm(ctx: &Context, input: &str) -> anyhow::Result<String> {
  let url = format!("{}/{}", ctx.env.atlaspack_npm_url, input);
  let api_response = http::download_serde::<NpmApiResponse>(&url)?;
  Ok(api_response.version)
}

pub fn install_from_npm(
  ctx: Context,
  cmd: InstallCommand,
  version: &Specifier,
) -> anyhow::Result<()> {
  println!("Resolving");
  let pkg = NpmPackage::from_name(&ctx.paths.versions_v1, version)?;
  let url = format!("{}/{}", ctx.env.atlaspack_npm_url, version.version());
  let api_response = http::download_serde::<NpmApiResponse>(&url)?;

  let tarball_url = api_response.dist.tarball;
  let integrity = api_response.dist.integrity;

  println!("Downloading");
  let bytes = http::download_bytes(&tarball_url)?;

  if !Integrity::parse(&integrity)?.eq(&bytes) {
    return Err(anyhow::anyhow!("Integrity check failed"));
  }

  println!("Extracting");
  let temp_dir = ctx.paths.temp_dir()?;
  archive::tar_gz(bytes.as_slice()).unpack(&temp_dir)?;
  let inner_temp = fs::read_dir(&temp_dir)?.try_first()?;

  println!("Finalizing");
  fs_ext::create_dir_if_not_exists(&pkg.path)?;
  fs_ext::create_dir_if_not_exists(pkg.contents())?;

  PackageMeta::write_to_file(
    &PackageMeta {
      kind: PackageKind::Npm,
      version: Some(version.version().to_string()),
      specifier: Some(version.to_string()),
      checksum: Some(integrity),
    },
    pkg.meta_file(),
  )?;

  fs::rename(inner_temp.path(), pkg.contents())?;

  if cmd.skip_postinstall {
    return Ok(());
  }

  // If the git sha is specified, use the yarn.lock from the repo
  if fs::exists(pkg.contents().join("GIT_SHA"))? {
    let git_sha = fs::read_to_string(pkg.contents().join("GIT_SHA"))?;
    let url_lock_file = format!("{}/{}/{}", c::GITHUB_RAW, git_sha, "yarn.lock");
    let bytes = http::download_bytes(url_lock_file)?;
    fs::write(pkg.contents().join("yarn.lock"), bytes)?;
  }

  let node_path = resolve_executable("node")?;
  let pkg_manager_path = match resolve_executable("yarn") {
    Ok(yarn) => Ok(yarn),
    Err(_) => resolve_executable("npm"),
  }?;

  exec_blocking(
    [
      node_path.try_to_string()?.as_str(),
      pkg_manager_path.try_to_string()?.as_str(),
      "install",
    ],
    &ExecOptions {
      cwd: Some(pkg.contents()),
      stdio: true,
      env: None,
    },
  )?;

  Ok(())
}
