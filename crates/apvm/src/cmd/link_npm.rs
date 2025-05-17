use std::fs;
use std::fs::Permissions;

use log::info;

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
  let package_lib_contents = package_contents.join("lib");

  // node_modules
  let node_modules = ctx.env.pwd.join("node_modules");
  let node_modules_bin = node_modules.join(".bin");
  let node_modules_apvm = node_modules.join(".apvm");

  // node_modules/.bin
  #[cfg(unix)]
  let node_modules_bin_atlaspack = node_modules_bin.join("atlaspack");
  #[cfg(windows)]
  let node_modules_bin_atlaspack = node_modules_bin.join("atlaspack.exe");

  // node_modules/atlaspack
  let node_modules_super = node_modules.join("atlaspack");

  // node_modules/@atlaspack
  let node_modules_atlaspack = node_modules.join("@atlaspack");

  // Create node_modules if it doesn't exist
  if !fs::exists(&node_modules)? {
    info!("Deleting: {:?}", node_modules);
    fs::create_dir_all(&node_modules)?;
  }
  if !fs::exists(&node_modules_bin)? {
    info!("Deleting: {:?}", node_modules_bin);
    fs::create_dir_all(&node_modules_bin)?;
  }

  // Delete existing node_modules/.bin/atlaspack
  if fs::exists(&node_modules_bin_atlaspack)? {
    info!("Deleting: {:?}", node_modules_bin_atlaspack);
    fs::remove_file(&node_modules_bin_atlaspack)?;
  }

  // node_modules/atlaspack
  if fs::exists(&node_modules_super)? {
    info!("Deleting: {:?}", node_modules_super);
    fs::remove_dir_all(&node_modules_super)?;
  }
  fs::create_dir_all(&node_modules_super)?;

  // node_modules/.apvm
  if fs::exists(&node_modules_apvm)? {
    info!("Deleting: {:?}", node_modules_apvm);
    fs::remove_dir_all(&node_modules_apvm)?
  }
  fs::create_dir_all(&node_modules_apvm)?;

  // Create node_modules/@atlaspack
  if fs::exists(&node_modules_atlaspack)? {
    info!("Deleting: {:?}", node_modules_atlaspack);
    for entry in fs::read_dir(&node_modules_atlaspack)? {
      let entry = entry?;
      let entry_path = entry.path();
      let file_stem = entry_path.try_file_stem()?;
      if file_stem.contains("apvm") {
        continue;
      }
      fs::remove_dir_all(&entry_path)?;
    }
  } else {
    fs::create_dir_all(&node_modules_atlaspack)?;
  }

  info!("Copying: {:?} {:?}", package_contents, node_modules_super);

  fs_ext::cp_dir_recursive(&package_contents, &node_modules_super)?;

  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;

    info!("Creating: node_modules/.bin/atlaspack");
    fs::write(
      &node_modules_bin_atlaspack,
      "#!/usr/bin/env node\nrequire('atlaspack/lib/cli.js')\n",
    )?;
    fs::set_permissions(&node_modules_bin_atlaspack, Permissions::from_mode(0o777))?;
  }

  #[cfg(windows)]
  {
    // Just use a wrapper process for Windows
    crate::platform::fs_ext::hard_link_or_copy(&ctx.env.exe_path, &node_modules_bin_atlaspack)?;
  }

  for entry in fs::read_dir(&package_lib_contents)? {
    let entry = entry?;
    let entry_path = entry.path();

    info!("Entry: {:?}", entry_path);

    if fs::metadata(&entry_path)?.is_dir() {
      continue;
    }

    // Skip non .js files
    if entry_path.extension().is_some_and(|ext| ext != "js") {
      continue;
    }

    let file_stem = entry_path.try_file_stem()?;
    if file_stem.starts_with("vendor.") {
      continue;
    }

    let node_modules_atlaspack_pkg = node_modules_atlaspack.join(&file_stem);
    info!(
      "Linking: {:?} -> {:?}",
      file_stem, node_modules_atlaspack_pkg
    );
    if fs::exists(&node_modules_atlaspack_pkg)? {
      info!("Deleting: {:?}", node_modules_atlaspack_pkg);
      fs::remove_dir_all(&node_modules_atlaspack_pkg)?;
    }

    fs::create_dir(&node_modules_atlaspack_pkg)?;
    fs::write(
      node_modules_atlaspack_pkg.join("package.json"),
      (json::object! {
        "name": format!("@atlaspack/{file_stem}"),
        "main": "./index.js",
        "type": "commonjs"
      })
      .pretty(2),
    )?;

    // TODO: Remove this. One of our consumer packages imports this path directly.
    // This path should not be imported by consumers and will be fixed there.
    // In the interim, this workaround unblocks the roll out but will eventually be removed.
    if file_stem == "runtime-js" {
      fs::create_dir_all(node_modules_atlaspack_pkg.join("lib").join("helpers"))?;
      fs_ext::soft_link(
        &package_lib_contents
          .join("runtimes")
          .join("js")
          .join("helpers")
          .join("bundle-manifest.js"),
        &node_modules_atlaspack_pkg
          .join("lib")
          .join("helpers")
          .join("bundle-manifest.js"),
      )?;
    }

    fs::write(
      node_modules_atlaspack_pkg.join("index.js"),
      format!("module.exports = require('atlaspack/lib/{file_stem}.js')\n"),
    )?;

    fs::write(
      node_modules_atlaspack_pkg.join("index.d.ts"),
      format!(
        r#"// @ts-ignore
export * from "atlaspack/{file_stem}";
// @ts-ignore
export {{ default }} from "atlaspack/{file_stem}";
"#
      ),
    )?;
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
    ctx.validator.set_alias(&alias, &specifier.to_string())?;
    ctx
      .validator
      .update_checksum(specifier, package.checksum.unwrap_or_default())?;
  }

  Ok(())
}
