use std::fs;
use std::fs::Permissions;

use fs_extra::dir::CopyOptions;
use log::info;

use super::link::LinkCommand;
use crate::context::Context;
use crate::platform::constants as c;
use crate::platform::link;
use crate::platform::name;
use crate::platform::package_path::PackagePath;
use crate::platform::path_ext::*;

pub fn link_npm(ctx: Context, cmd: LinkCommand) -> anyhow::Result<()> {
  // let version_decoded = cmd.version.unwrap();
  // let version = name::encode(&version_decoded)?;
  // // let package_lib = package.path_real()?;
  // let package_path = ctx.paths.versions_v1_pkg(&version);
  // let package_lib_contents = package_path.contents.join("lib");

  // // node_modules
  // let node_modules = ctx.env.pwd.join("node_modules");
  // let node_modules_bin = node_modules.join(".bin");
  // let node_modules_apvm = node_modules.join(".apvm");

  // // node_modules/.bin
  // #[cfg(unix)]
  // let node_modules_bin_atlaspack = node_modules_bin.join("atlaspack");
  // #[cfg(windows)]
  // let node_modules_bin_atlaspack = node_modules_bin.join("atlaspack.exe");

  // // node_modules/atlaspack
  // let node_modules_super = node_modules.join("atlaspack");

  // // node_modules/@atlaspack
  // let node_modules_atlaspack = node_modules.join("@atlaspack");

  // // Create node_modules if it doesn't exist
  // if !fs::exists(&node_modules)? {
  //   info!("Deleting: {:?}", node_modules);
  //   fs::create_dir_all(&node_modules)?;
  // }
  // if !fs::exists(&node_modules_bin)? {
  //   info!("Deleting: {:?}", node_modules_bin);
  //   fs::create_dir_all(&node_modules_bin)?;
  // }

  // // Delete existing node_modules/.bin/atlaspack
  // if fs::exists(&node_modules_bin_atlaspack)? {
  //   info!("Deleting: {:?}", node_modules_bin_atlaspack);
  //   fs::remove_file(&node_modules_bin_atlaspack)?;
  // }

  // // node_modules/atlaspack
  // if fs::exists(&node_modules_super)? {
  //   info!("Deleting: {:?}", node_modules_super);
  //   fs::remove_dir_all(&node_modules_super)?;
  // }
  // fs::create_dir_all(&node_modules_super)?;

  // // node_modules/.apvm
  // if fs::exists(&node_modules_apvm)? {
  //   info!("Deleting: {:?}", node_modules_apvm);
  //   fs::remove_dir_all(&node_modules_apvm)?
  // }
  // fs::create_dir_all(&node_modules_apvm)?;

  // // Create node_modules/@atlaspack
  // if fs::exists(&node_modules_atlaspack)? {
  //   info!("Deleting: {:?}", node_modules_atlaspack);
  //   for entry in fs::read_dir(&node_modules_atlaspack)? {
  //     let entry = entry?;
  //     let entry_path = entry.path();
  //     let file_stem = entry_path.try_file_stem()?;
  //     if file_stem.contains("apvm") {
  //       continue;
  //     }
  //     fs::remove_dir_all(&entry_path)?;
  //   }
  // } else {
  //   fs::create_dir_all(&node_modules_atlaspack)?;
  // }

  // info!(
  //   "Copying: {:?} {:?}",
  //   package_lib_contents, node_modules_super
  // );

  // fs_extra::dir::copy(
  //   &package_lib_contents,
  //   &node_modules_super,
  //   &CopyOptions {
  //     content_only: true,
  //     ..Default::default()
  //   },
  // )?;

  // #[cfg(unix)]
  // {
  //   use std::os::unix::fs::PermissionsExt;

  //   info!("Creating: node_modules/.bin/atlaspack");
  //   fs::write(
  //     &node_modules_bin_atlaspack,
  //     "#!/usr/bin/env node\nrequire('atlaspack/lib/cli.js')\n",
  //   )?;
  //   fs::set_permissions(&node_modules_bin_atlaspack, Permissions::from_mode(0o777))?;
  // }

  // #[cfg(windows)]
  // {
  //   // Just use a wrapper process for Windows
  //   crate::platform::link::hard_link_or_copy(&ctx.env.exe_path, &node_modules_bin_atlaspack)?;
  // }

  // for entry in fs::read_dir(&package_lib_contents)? {
  //   let entry = entry?;
  //   let entry_path = entry.path();

  //   info!("Entry: {:?}", entry_path);

  //   if fs::metadata(&entry_path)?.is_dir() {
  //     continue;
  //   }

  //   // Skip non .js files
  //   if entry_path.extension().is_some_and(|ext| ext != "js") {
  //     continue;
  //   }

  //   let file_stem = entry_path.try_file_stem()?;
  //   if file_stem.starts_with("vendor.") {
  //     continue;
  //   }

  //   let node_modules_atlaspack_pkg = node_modules_atlaspack.join(&file_stem);
  //   info!(
  //     "Linking: {:?} -> {:?}",
  //     file_stem, node_modules_atlaspack_pkg
  //   );
  //   if fs::exists(&node_modules_atlaspack_pkg)? {
  //     info!("Deleting: {:?}", node_modules_atlaspack_pkg);
  //     fs::remove_dir_all(&node_modules_atlaspack_pkg)?;
  //   }

  //   fs::create_dir(&node_modules_atlaspack_pkg)?;
  //   fs::write(
  //     node_modules_atlaspack_pkg.join("package.json"),
  //     (json::object! {
  //       "name": format!("@atlaspack/{file_stem}"),
  //       "main": "./index.js",
  //       "type": "commonjs"
  //     })
  //     .pretty(2),
  //   )?;

  //   // TODO: Remove this. One of our consumer packages imports this path directly.
  //   // This path should not be imported by consumers and will be fixed there.
  //   // In the interim, this workaround unblocks the roll out but will eventually be removed.
  //   if file_stem == "runtime-js" {
  //     fs::create_dir_all(node_modules_atlaspack_pkg.join("lib").join("helpers"))?;
  //     link::soft_link(
  //       &package_lib_contents
  //         .join("runtimes")
  //         .join("js")
  //         .join("helpers")
  //         .join("bundle-manifest.js"),
  //       &node_modules_atlaspack_pkg
  //         .join("lib")
  //         .join("helpers")
  //         .join("bundle-manifest.js"),
  //     )?;
  //   }

  //   fs::write(
  //     node_modules_atlaspack_pkg.join("index.js"),
  //     format!("module.exports = require('atlaspack/lib/{file_stem}.js')\n"),
  //   )?;
  // }

  // LinkedMeta {
  //   kind: VersionKind::Npm,
  //   version_specifier: Some(version_decoded),
  //   original_path: PackagePath::Installed(package_path),
  // }
  // .write_to_file(node_modules_apvm.join(c::APVM_VERSION_FILE))?;
  // // fs::write(
  // //   node_modules_apvm.join(c::APVM_VERSION_FILE),
  // //   package.version.to_string().as_bytes(),
  // // )?;
  Ok(())
}
