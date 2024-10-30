mod package_json;
mod target;

pub use target::*;

// use std::path::PathBuf;
// use std::sync::Arc;

// use super::ActionQueue;
// use crate::compilation::Compilation;

// #[derive(Debug)]
// pub struct TargetAction {}

// impl TargetAction {
//   pub async fn run(
//     self,
//     c: Arc<Compilation>,
//     q: ActionQueue,
//   ) -> anyhow::Result<()> {
//     Ok(())
//   }
// }
