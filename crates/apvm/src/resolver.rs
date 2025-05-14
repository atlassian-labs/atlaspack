// pub fn resolve<S: AsRef<str>>(
//   &self,
//   specifier: &Option<S>,
// ) -> anyhow::Result<VersionResolveResult> {
//   match self.resolve_local(specifier) {
//     VersionResolveResult::Installed(version) => {
//       return Ok(VersionResolveResult::Installed(version))
//     }
//     VersionResolveResult::NotInstalledLocal => {
//       return Ok(VersionResolveResult::NotInstalledLocal)
//     }
//     _ => {}
//   };

//   match self.resolve_alias(specifier)? {
//     VersionResolveResult::Installed(version) => {
//       return Ok(VersionResolveResult::Installed(version))
//     }
//     VersionResolveResult::NotInstalled { specifier, kind } => {
//       return Ok(VersionResolveResult::NotInstalled { specifier, kind })
//     }
//     _ => {}
//   };

//   let Some(specifier) = specifier.as_ref() else {
//     return Ok(VersionResolveResult::CannotResolve);
//   };

//   match self.resolve_installed(specifier)? {
//     VersionResolveResult::Installed(version) => {
//       return Ok(VersionResolveResult::Installed(version))
//     }
//     VersionResolveResult::NotInstalled { specifier, kind } => {
//       return Ok(VersionResolveResult::NotInstalled { specifier, kind })
//     }
//     _ => {}
//   };

//   Ok(VersionResolveResult::CannotResolve)
// }

// pub fn resolve_alias<S: AsRef<str>>(
//   &self,
//   specifier: &Option<S>,
// ) -> anyhow::Result<VersionResolveResult> {
//   let Some(apvmrc) = &self.apvmrc else {
//     return Ok(VersionResolveResult::CannotResolve);
//   };

//   let Some(specifier) = specifier.as_ref() else {
//     let Some(default) = apvmrc.version_aliases.get("default") else {
//       return Ok(VersionResolveResult::CannotResolve);
//     };

//     let Some(package_specifier) = &default.specifier else {
//       return Err(anyhow::anyhow!("Alias has undefined version"));
//     };

//     return self.resolve_installed(package_specifier);
//   };

//   let specifier = specifier.as_ref();

//   for (alias, package_meta) in &apvmrc.version_aliases {
//     if specifier == alias {
//       let Some(package_specifier) = &package_meta.specifier else {
//         return Err(anyhow::anyhow!("Alias has undefined version"));
//       };
//       return self.resolve_installed(package_specifier);
//     }
//   }

//   Ok(VersionResolveResult::CannotResolve)
// }

// pub fn resolve_installed<S: AsRef<str>>(
//   &self,
//   specifier: S,
// ) -> anyhow::Result<VersionResolveResult> {
//   let (kind, specifier) = parse_specifier(specifier)?;

//   for version in &self.installed {
//     if version.kind == kind && version.specifier == specifier {
//       return Ok(match version.kind {
//         PackageKind::Npm => VersionResolveResult::Installed(version.clone()),
//         PackageKind::Git => VersionResolveResult::Installed(version.clone()),
//         PackageKind::Local => VersionResolveResult::Invalid,
//         PackageKind::Unmanaged => VersionResolveResult::Invalid,
//       });
//     }
//   }

//   Ok(VersionResolveResult::NotInstalled { specifier, kind })
// }

// pub fn resolve_local<S: AsRef<str>>(&self, specifier: &Option<S>) -> VersionResolveResult {
//   let Some(specifier) = specifier else {
//     return VersionResolveResult::CannotResolve;
//   };

//   let Some(local) = &self.local else {
//     return VersionResolveResult::NotInstalledLocal;
//   };

//   if specifier.as_ref() != "local" {
//     return VersionResolveResult::CannotResolve;
//   }

//   VersionResolveResult::Installed(local.clone())
// }
