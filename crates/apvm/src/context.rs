use serde::Serialize;

use crate::env::Env;
use crate::paths::Paths;
use crate::platform::apvmrc::ApvmRc;
use crate::resolver::PackageResolver;
use crate::versions::Versions;

#[derive(Debug, Clone, Serialize)]
pub struct Context {
  pub env: Env,
  pub paths: Paths,
  pub apvmrc: ApvmRc,
  pub versions: Versions,
  #[serde(skip)]
  pub resolver: PackageResolver,
}
