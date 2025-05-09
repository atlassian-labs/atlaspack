use std::fs;
use std::fs::Permissions;

use log::info;

use super::link::LinkCommand;
use crate::context::Context;
use crate::platform::constants as c;
use crate::platform::link;
use crate::platform::package::PackageDescriptor;
use crate::platform::package_json::PackageJson;
use crate::platform::path_ext::*;

pub fn link_local(
  ctx: Context,
  _cmd: LinkCommand,
  package: PackageDescriptor,
) -> anyhow::Result<()> {
  let package_lib = package.path_real()?;
  let package_packages = package_lib.join("packages");

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

  // Create node_modules/@atlaspack
  if fs::exists(&node_modules_atlaspack)? {
    info!("Deleting: {:?}", node_modules_atlaspack);
    fs::remove_dir_all(&node_modules_atlaspack)?;
  }
  fs::create_dir_all(&node_modules_atlaspack)?;

  // node_modules/.apvm
  if fs::exists(&node_modules_apvm)? {
    info!("Deleting: {:?}", node_modules_apvm);
    fs::remove_dir_all(&node_modules_apvm)?
  }
  fs::create_dir_all(&node_modules_apvm)?;

  for entry in fs::read_dir(&package_packages)? {
    let entry = entry?;
    let entry_path = entry.path();

    for entry in fs::read_dir(&entry_path)? {
      let entry = entry?;
      let entry_path = entry.path();

      if entry_path.try_to_string()?.contains("examples")
        || entry_path.try_to_string()?.contains("dev")
        || entry_path.try_to_string()?.contains("apvm")
        || entry_path.try_to_string()?.ends_with("atlaspack")
      {
        continue;
      }

      let package_json_path = entry_path.join("package.json");
      if !fs::exists(&package_json_path)? {
        continue;
      }
      let package_json_str = fs::read_to_string(&package_json_path)?;
      let package_json = serde_json::from_str::<PackageJson>(&package_json_str)?;

      if package_json.private.is_some_and(|v| v) {
        continue;
      }

      let Some(name) = package_json.name else {
        continue;
      };

      let name = name.replace("@atlaspack/", "");
      log::info!("Linking: {}", name);

      link::soft_link(&entry_path, &node_modules_atlaspack.join(&name))?;
    }
  }

  link::soft_link(
    &package_lib.join("node_modules"),
    &node_modules_atlaspack.join("node_modules"),
  )?;

  fs::write(
    &node_modules_bin_atlaspack,
    r#"#!/usr/bin/env node

if (process.env.ATLASPACK_DEV === 'true') {{
  console.log('USING ATLASPACK SOURCES')
  require('@atlaspack/babel-register')
  require('@atlaspack/cli/src/bin.js')
}} else {{
  require('@atlaspack/cli/bin/atlaspack.js')
}}
"#,
  )?;

  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&node_modules_bin_atlaspack, Permissions::from_mode(0o777))?;
  }

  fs::write(node_modules_apvm.join(c::APVM_VERSION_FILE), r"local")?;

  Ok(())
}
