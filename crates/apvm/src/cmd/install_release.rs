use flate2::read::GzDecoder;
use tar::Archive;

use super::install::InstallCommand;
use crate::context::Context;
use crate::platform::hash::Integrity;
use crate::platform::package::ReleasePackage;
use crate::platform::specifier::Specifier;
use crate::platform::{constants as c, fs_ext, http};
use crate::public::json_serde::JsonSerde;
use crate::public::package_kind::PackageKind;
use crate::public::package_meta::PackageMeta;

pub fn install_from_release(
  ctx: Context,
  cmd: InstallCommand,
  version: &Specifier,
) -> anyhow::Result<()> {
  let pkg = ReleasePackage::from_name(&ctx.paths.versions_v1, version)?;

  println!("Downloading");
  let url_tarball = format!(
    "{}/v{}/{}.tar.gz",
    c::RELEASE_URL,
    version.version(),
    c::RELEASE_NAME
  );
  let url_checksum = format!("{}.integrity", url_tarball);
  let bytes_archive = http::download_bytes(&url_tarball)?;
  let checksum = http::download_string(&url_checksum)?;

  if !cmd.skip_checksum && !Integrity::parse(&checksum)?.eq(&bytes_archive) {
    return Err(anyhow::anyhow!("Integrity check failed"));
  }

  println!("Extracting");
  let tar = GzDecoder::new(bytes_archive.as_slice());
  let mut archive = Archive::new(tar);

  fs_ext::create_dir_if_not_exists(pkg.contents())?;
  archive.unpack(pkg.contents())?;

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
