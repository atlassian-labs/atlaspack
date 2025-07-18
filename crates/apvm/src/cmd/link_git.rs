use std::fs;
use std::fs::Permissions;
use std::path::Path;

use super::link::LinkCommand;
use crate::context::Context;
use crate::platform::constants as c;
use crate::platform::fs_ext;
use crate::platform::package::GitPackage;
use crate::platform::path_ext::*;
use crate::platform::specifier::Specifier;
use crate::public::json_serde::JsonSerde;
use crate::public::linked_meta::LinkedMeta;
use crate::public::package_json::PackageJson;
use crate::public::package_kind::PackageKind;

pub fn link_git(
  ctx: Context,
  _cmd: LinkCommand,
  specifier: &Specifier,
  package: GitPackage,
) -> anyhow::Result<()> {
  let package_contents = package.contents();
  let package_packages = package_contents.join("packages");

  // node_modules
  let node_modules = ctx.env.pwd.join("node_modules");
  let node_modules_bin = node_modules.join(".bin");
  let node_modules_apvm = node_modules.join(".apvm");
  let node_modules_bin_atlaspack = node_modules_bin.join(c::NM_BIN_NAME);
  let node_modules_atlaspack = node_modules.join("@atlaspack");

  // Create the following folder structure
  //   /node_modules
  //     /.bin
  //       atlaspack
  //     /.apvm
  //     /@atlaspack
  //     /atlaspack

  // Create node_modules if it doesn't exist
  fs_ext::create_dir_if_not_exists(&node_modules)?;
  fs_ext::create_dir_if_not_exists(&node_modules_bin)?;

  // node_modules/.apvm
  fs_ext::recreate_dir(&node_modules_apvm)?;

  // Delete existing node_modules/.bin/atlaspack
  fs_ext::remove_if_exists(&node_modules_bin_atlaspack)?;
  create_bin(&node_modules_bin_atlaspack)?;

  // Create node_modules/@atlaspack
  // Remove internal folders excluding apvm
  fs_ext::create_dir_if_not_exists(&node_modules_atlaspack)?;
  for entry in fs::read_dir(&node_modules_atlaspack)? {
    let entry_path = entry?.path();
    if entry_path.try_file_stem()?.contains("apvm") {
      continue;
    }
    fs_ext::remove_if_exists(&entry_path)?;
  }

  // Map packages to target directory
  for entry in fs::read_dir(&package_packages)? {
    let entry = entry?;
    let entry_path = entry.path();
    let basename = entry_path.try_file_name()?;

    if should_skip(&basename) {
      continue;
    }

    for entry in fs::read_dir(&entry_path)? {
      let entry = entry?;
      let entry_path = entry.path();
      let basename = entry_path.try_file_name()?;

      if should_skip(&basename) {
        continue;
      }

      let package_json_path = entry_path.join("package.json");
      if !fs::exists(&package_json_path)? {
        continue;
      }

      let package_json = PackageJson::read_from_file(&package_json_path)?;

      if package_json.private.is_some_and(|v| v) {
        continue;
      }

      let Some(name) = package_json.name else {
        continue;
      };

      let name = name.replace("@atlaspack/", "");
      log::info!("Linking: {}", name);

      let pkg_dir = node_modules_atlaspack.join(&name);
      fs::create_dir_all(&pkg_dir)?;
      fs_ext::cp_dir_recursive(&entry_path, node_modules_atlaspack.join(&pkg_dir))?;
    }
  }

  fs_ext::cp_dir_recursive(
    package_contents.join("node_modules"),
    node_modules_atlaspack.join("node_modules"),
  )?;

  LinkedMeta::write_to_file(
    &LinkedMeta {
      kind: PackageKind::Git,
      version: Some(specifier.version().to_string()),
      specifier: Some(specifier.to_string()),
      original_path: package.path.clone(),
      content_path: package.path,
    },
    node_modules_apvm.join(c::LINK_META_FILE),
  )?;

  Ok(())
}

fn create_bin(node_modules_bin_atlaspack: &Path) -> std::io::Result<()> {
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    fs::write(
      node_modules_bin_atlaspack,
      "#!/usr/bin/env node\nrequire('@atlaspack/cli/bin/atlaspack.js')\n",
    )?;
    fs::set_permissions(node_modules_bin_atlaspack, Permissions::from_mode(0o777))?;
  }

  #[cfg(windows)]
  {
    // Just use a wrapper process for Windows
    crate::platform::fs_ext::hard_link_or_copy(&ctx.env.exe_path, &node_modules_bin_atlaspack)?;
  }

  Ok(())
}

fn should_skip(entry: &str) -> bool {
  if entry.contains("examples")
    || entry.contains("dev")
    || entry.contains("apvm")
    || entry.contains("atlaspack")
  {
    return true;
  }
  false
}
