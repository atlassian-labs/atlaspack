use swc_core::ecma::ast::Program;
use swc_core::ecma::visit::VisitMutWith;

use crate::strip_runtime::StripRuntimeTransform;

#[allow(unused_imports)]
pub use crate::types::{
  ExtractStylesToDirectory, PluginOptions, TransformConfig, TransformMetadata, TransformOutput,
};

use crate::types::{
  TransformConfig as LocalTransformConfig, TransformMetadata as LocalTransformMetadata,
  TransformOutput as LocalTransformOutput,
};

/// Entry point mirroring the Babel strip-runtime plugin API.
pub fn transform(program: Program, config: LocalTransformConfig) -> LocalTransformOutput {
  let mut transform = StripRuntimeTransform::new(config);
  let mut program = program;
  program.visit_mut_with(&mut transform);

  let metadata: LocalTransformMetadata = transform.into_metadata();

  LocalTransformOutput { program, metadata }
}
