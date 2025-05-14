use super::package_kind::PackageKind;

pub fn parse_specifier<S: AsRef<str>>(
  specifier: S,
) -> anyhow::Result<(PackageKind, Option<String>)> {
  let specifier = specifier.as_ref();

  if let Some((origin, specifier)) = specifier.split_once(":") {
    return match origin {
      "npm" => Ok((PackageKind::Npm, Some(specifier.to_string()))),
      "git" => Ok((PackageKind::Git, Some(specifier.to_string()))),
      "local" => Ok((PackageKind::Local, None)),
      _ => Err(anyhow::anyhow!("Cannot parse version specifier")),
    };
  };

  if specifier == "local" {
    return Ok((PackageKind::Local, None));
  }

  Ok((PackageKind::Npm, Some(specifier.to_string())))
}
