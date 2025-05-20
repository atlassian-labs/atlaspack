use std::fs;
use std::fs::Permissions;
use std::path::Path;

use super::link::LinkCommand;
use crate::context::Context;
use crate::platform::constants as c;
use crate::platform::fs_ext;
use crate::platform::package::ReleasePackage;
use crate::platform::path_ext::*;
use crate::platform::specifier::Specifier;
use crate::public::json_serde::JsonSerde;
use crate::public::linked_meta::LinkedMeta;
use crate::public::package_json::PackageJson;
use crate::public::package_kind::PackageKind;

pub fn link_release(
  ctx: Context,
  cmd: LinkCommand,
  specifier: &Specifier,
  package: ReleasePackage,
) -> anyhow::Result<()> {
  // Directories
  // /<version>/contents
  let dir_package = package.contents();
  // /<version>/contents/node_modules/@atlaspack
  let dir_package_node_modules_atlaspack = dir_package.join("node_modules").join("@atlaspack");

  // $PWD/node_modules
  let node_modules = ctx.env.pwd.join("node_modules");
  let node_modules_bin = node_modules.join(".bin");
  let node_modules_bin_atlaspack = node_modules_bin.join(c::NM_BIN_NAME);
  let node_modules_apvm = node_modules.join(".apvm");
  let node_modules_super = node_modules.join("atlaspack");
  let node_modules_atlaspack = node_modules.join("@atlaspack");

  // Create the following folder structure
  //   /node_modules
  //     /.apvm
  //     /@atlaspack
  //     /atlaspack

  // Create node_modules if it doesn't exist
  fs_ext::create_dir_if_not_exists(&node_modules)?;
  fs_ext::create_dir_if_not_exists(&node_modules_bin)?;

  // Recreate node_modules/atlaspack
  fs_ext::recreate_dir(&node_modules_super)?;

  // Recreate node_modules/.apvm
  fs_ext::recreate_dir(&node_modules_apvm)?;

  // Copy managed version into node_modules
  fs_ext::cp_dir_recursive(&dir_package, &node_modules_super)?;

  // Delete existing node_modules/.bin/atlaspack
  fs_ext::remove_if_exists(&node_modules_bin_atlaspack)?;
  create_bin(&node_modules_bin_atlaspack)?;

  // Recreate node_modules/@atlaspack
  fs_ext::create_dir_if_not_exists(&node_modules_atlaspack)?;
  for entry in fs::read_dir(&node_modules_atlaspack)? {
    let entry_path = entry?.path();
    if entry_path.try_file_stem()?.contains("apvm") {
      continue;
    }
    fs_ext::remove_if_exists(&entry_path)?;
  }

  // Map super package to target directory
  for entry in fs::read_dir(&dir_package_node_modules_atlaspack)? {
    let entry = entry?;
    let entry_path = entry.path();
    let entry_basename = entry.file_name().try_to_string()?;
    let dest = node_modules_atlaspack.join(&entry_basename);

    log::info!("creating_shim_for: {:?}", entry_path);
    fs_ext::recreate_dir(&dest)?;
    PackageJson::write_to_file(
      &PackageJson {
        name: Some(format!("@atlaspack/{entry_basename}")),
        version: Some(specifier.to_string()),
        main: Some("./index.js".to_string()),
        types: Some("./index.d.ts".to_string()),
        r#type: Some("commonjs".to_string()),
        ..PackageJson::read_from_file(entry_path.join("package.json"))?
      },
      dest.join("package.json"),
    )?;

    fs::write(
      dest.join("index.js"),
      format!("module.exports = require('atlaspack/{}')\n", entry_basename),
    )?;

    fs::write(
      dest.join("index.d.ts"),
      format!(
        r#"// @ts-ignore
export * from "atlaspack/{}";
// @ts-ignore
export {{ default }} from "atlaspack/{}";
"#,
        entry_basename, entry_basename,
      ),
    )?;
  }

  LinkedMeta::write_to_file(
    &LinkedMeta {
      kind: PackageKind::Release,
      version: Some(specifier.version().to_string()),
      specifier: Some(specifier.to_string()),
      original_path: package.path.clone(),
      content_path: package.contents(),
    },
    node_modules_apvm.join(c::LINK_META_FILE),
  )?;

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
    log::info!(
      "creating: {:?}",
      fs_ext::append_ext("exe", node_modules_bin_atlaspack)
    );

    fs::write(
      node_modules_bin_atlaspack,
      "#!/usr/bin/env node\nrequire('atlaspack/bin')\n",
    )?;

    fs::set_permissions(node_modules_bin_atlaspack, Permissions::from_mode(0o777))?;
  }

  #[cfg(windows)]
  {
    let bin_path = fs_ext::append_ext("ps1", node_modules_bin_atlaspack);

    fs::write(&bin_path, r#"Console-Write "todo""#)?;
  }
  Ok(())
}
