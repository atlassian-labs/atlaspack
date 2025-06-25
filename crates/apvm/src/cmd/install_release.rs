use super::install::InstallCommand;
use crate::context::Context;
use crate::platform::archive;
use crate::platform::constants as c;
use crate::platform::fs_ext;
use crate::platform::hash::Integrity;
use crate::platform::http;
use crate::platform::package::ReleasePackage;
use crate::platform::specifier::Specifier;
use crate::public::json_serde::JsonSerde;
use crate::public::package_kind::PackageKind;
use crate::public::package_meta::PackageMeta;

pub fn install_from_release(
  ctx: Context,
  _cmd: InstallCommand,
  version: &Specifier,
) -> anyhow::Result<()> {
  let pkg = ReleasePackage::from_name(&ctx.paths.versions_v1, version)?;

  println!("Downloading");
  let url_tarball = format!(
    "{}/{}/{}.tar.xz",
    ctx.env.atlaspack_release_url,
    version.version(),
    c::RELEASE_NAME_UNIVERSAL
  );
  let url_checksum = format!("{}.sha512", url_tarball);
  let bytes_archive = http::download_bytes(&url_tarball)?;
  let checksum = http::download_string(&url_checksum)?;

  if !Integrity::parse(&checksum)?.eq(&bytes_archive) {
    return Err(anyhow::anyhow!("Integrity check failed"));
  }

  println!("Extracting");
  fs_ext::create_dir_if_not_exists(pkg.contents())?;
  archive::tar_xz(bytes_archive.as_slice()).unpack(pkg.contents())?;

  PackageMeta::write_to_file(
    &PackageMeta {
      kind: PackageKind::Release,
      version: Some(version.version().to_string()),
      specifier: Some(version.to_string()),
      checksum: Some(checksum),
    },
    pkg.meta_file(),
  )?;

  Ok(())
}
