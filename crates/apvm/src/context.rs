use serde::Serialize;

use crate::env::Env;
use crate::paths::Paths;
use crate::platform::apvmrc::ApvmRcRef;
use crate::resolver::PackageResolver;
use crate::validator::Validator;
use crate::versions::Versions;

#[derive(Debug, Clone, Serialize)]
pub struct Context {
  pub env: Env,
  pub paths: Paths,
  pub apvmrc: ApvmRcRef,
  pub versions: Versions,
  #[serde(skip)]
  pub resolver: PackageResolver,
  #[serde(skip)]
  pub validator: Validator,
}

impl Context {}
