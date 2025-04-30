use crate::env::Env;
use crate::paths::Paths;
use crate::platform::active::ActiveVersion;
use crate::platform::apvmrc::ApvmRc;

#[derive(Debug, Clone)]
pub struct Context {
  pub env: Env,
  pub apvmrc: Option<ApvmRc>,
  pub paths: Paths,
  pub active_version: Option<ActiveVersion>,
}
