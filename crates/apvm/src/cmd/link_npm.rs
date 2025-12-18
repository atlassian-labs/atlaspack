use std::fs;
use std::fs::Permissions;
use std::path::Path;

use super::link::LinkCommand;
use crate::context::Context;
use crate::platform::constants as c;
use crate::platform::fs_ext;
use crate::platform::package::NpmPackage;
use crate::platform::path_ext::*;
use crate::platform::specifier::Specifier;
use crate::public::json_serde::JsonSerde;
use crate::public::linked_meta::LinkedMeta;
use crate::public::package_kind::PackageKind;

pub fn link_npm(
  ctx: Context,
  cmd: LinkCommand,
  specifier: &Specifier,
  package: NpmPackage,
) -> anyhow::Result<()> {
  let package_contents = package.contents();
  let package_node_modules = package_contents.join("node_modules");
  let package_node_modules_atlaspack = package_node_modules.join("@atlaspack");

  // node_modules
  let node_modules = ctx.env.pwd.join("node_modules");
  let node_modules_bin = node_modules.join(".bin");
  let node_modules_apvm = node_modules.join(".apvm");
  let node_modules_bin_atlaspack = node_modules_bin.join(c::NM_BIN_NAME);
  let node_modules_super = node_modules.join("atlaspack");
  let node_modules_atlaspack = node_modules.join("@atlaspack");
  let node_modules_atlaspack_node_modules = node_modules_atlaspack.join("node_modules");

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

  // node_modules/atlaspack
  fs_ext::recreate_dir(&node_modules_super)?;

  // node_modules/.apvm
  fs_ext::recreate_dir(&node_modules_apvm)?;

  // Copy managed version into node_modules
  fs_ext::cp_dir_recursive(&package_contents, &node_modules_super)?;

  // Delete existing node_modules/.bin/atlaspack
  fs_ext::remove_if_exists(&node_modules_bin_atlaspack)?;
  create_bin(&node_modules_bin_atlaspack)?;

  // Create node_modules/@atlaspack
  fs_ext::create_dir_if_not_exists(&node_modules_atlaspack)?;
  for entry in fs::read_dir(&node_modules_atlaspack).map_err(|e| {
    anyhow::anyhow!(
      "Failed to read node_modules/@atlaspack directory {:?} in link_npm.rs: {}",
      node_modules_atlaspack,
      e
    )
  })? {
    let entry_path = entry?.path();
    if entry_path.try_file_stem()?.contains("apvm") {
      continue;
    }
    fs_ext::remove_if_exists(&entry_path)?;
  }

  // Map super package to target directory
  for entry in fs::read_dir(&package_node_modules_atlaspack).map_err(|e| {
    anyhow::anyhow!(
      "Failed to read package node_modules/@atlaspack directory {:?} in link_npm.rs: {}",
      package_node_modules_atlaspack,
      e
    )
  })? {
    let entry = entry?;
    let entry_path = entry.path();

    log::info!("Entry: {:?}", entry_path);

    if !fs::metadata(&entry_path)?.is_dir() {
      continue;
    }

    let file_stem = entry_path.try_file_stem()?;
    let node_modules_atlaspack_pkg = node_modules_atlaspack.join(&file_stem);
    log::info!(
      "Linking: {:?} -> {:?}",
      entry_path,
      node_modules_atlaspack_pkg
    );

    fs_ext::remove_if_exists(&node_modules_atlaspack_pkg)?;
    fs_ext::create_dir_if_not_exists(&node_modules_atlaspack_pkg)?;
    fs_ext::cp_dir_recursive(&entry_path, &node_modules_atlaspack_pkg)?;
  }

  if !cmd.exclude_node_modules {
    fs_ext::create_dir_if_not_exists(&node_modules_atlaspack_node_modules)?;
    fs_ext::cp_dir_recursive(&package_node_modules, &node_modules_atlaspack_node_modules)?;
    fs_ext::remove_if_exists(node_modules_atlaspack_node_modules.join("@atlaspack"))?;
  }

  let meta = LinkedMeta {
    kind: PackageKind::Npm,
    version: Some(specifier.version().to_string()),
    specifier: Some(specifier.to_string()),
    original_path: package.path.clone(),
    content_path: package.contents(),
  };

  meta.write_to_file(node_modules_apvm.join(c::LINK_META_FILE))?;

  if let Some(alias) = cmd.set_alias {
    println!("Updating config");
    ctx.apvmrc.set_alias(&alias, specifier.clone());
    ctx
      .apvmrc
      .set_checksum(specifier, package.checksum.unwrap_or_default());
    ctx.apvmrc.tidy();
    ctx.apvmrc.write_to_file()?;
  }

  Ok(())
}

fn create_bin(node_modules_bin_atlaspack: &Path) -> std::io::Result<()> {
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;

    log::info!("Creating: node_modules/.bin/atlaspack");
    fs::write(
      node_modules_bin_atlaspack,
      "#!/usr/bin/env node\nrequire('@atlaspack/cli')\n",
    )?;
    fs::set_permissions(node_modules_bin_atlaspack, Permissions::from_mode(0o777))?;
  }

  #[cfg(windows)]
  {
    // Just use a wrapper process for Windows
    // crate::platform::fs_ext::hard_link_or_copy(&ctx.env.exe_path, &node_modules_bin_atlaspack)?;
  }
  Ok(())
}
