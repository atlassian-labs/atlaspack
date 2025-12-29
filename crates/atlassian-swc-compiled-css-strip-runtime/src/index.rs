use swc_core::ecma::ast::Program;
use swc_core::ecma::visit::VisitMutWith;

use atlassian_swc_compiled_css::TransformResult;

use crate::strip_runtime::StripRuntimeTransform;

#[allow(unused_imports)]
pub use crate::types::{
  ExtractStylesToDirectory, PluginOptions, TransformConfig, TransformMetadata, TransformOutput,
};

use crate::types::{
  TransformConfig as LocalTransformConfig, TransformOutput as LocalTransformOutput,
};

/// Entry point mirroring the Babel strip-runtime plugin API.
pub fn transform(
  program: Program,
  config: LocalTransformConfig,
) -> TransformResult<LocalTransformOutput> {
  try_transform(program, config)
}

pub fn try_transform(
  program: Program,
  config: LocalTransformConfig,
) -> TransformResult<LocalTransformOutput> {
  let mut transform = StripRuntimeTransform::new(config);
  let mut program = program;
  program.visit_mut_with(&mut transform);

  let (metadata, errors) = transform.finish();

  if errors.is_empty() {
    Ok(LocalTransformOutput { program, metadata })
  } else {
    Err(errors)
  }
}
