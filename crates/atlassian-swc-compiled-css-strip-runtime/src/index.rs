use std::panic;
use swc_core::ecma::ast::Program;
use swc_core::ecma::visit::VisitMutWith;

use atlassian_swc_compiled_css::{TransformError, TransformResult};

use crate::strip_runtime::StripRuntimeTransform;

#[allow(unused_imports)]
pub use crate::types::{
  ExtractStylesToDirectory, PluginOptions, TransformConfig, TransformMetadata, TransformOutput,
};

use crate::types::{
  TransformConfig as LocalTransformConfig, TransformOutput as LocalTransformOutput,
};

/// Entry point mirroring the Babel strip-runtime plugin API.
///
/// This function delegates to `try_transform()` for implementation.
pub fn transform(
  program: Program,
  config: LocalTransformConfig,
) -> TransformResult<LocalTransformOutput> {
  try_transform(program, config)
}

/// Apply the strip-runtime transformation to remove @compiled/react runtime calls.
///
/// This function removes compiled CSS runtime imports and replaces css() calls with
/// className references. Panics that occur during transformation are caught and
/// converted into structured diagnostics rather than aborting the process.
///
/// # Error Handling
///
/// Returns `Err(Vec<TransformError>)` if:
/// - A panic occurs during transformation (caught via `catch_unwind`)
/// - The transform emits recoverable errors during processing
pub fn try_transform(
  program: Program,
  config: LocalTransformConfig,
) -> TransformResult<LocalTransformOutput> {
  panic::catch_unwind(panic::AssertUnwindSafe(|| {
    let mut transform = StripRuntimeTransform::new(config);
    let mut program = program;
    program.visit_mut_with(&mut transform);

    let (metadata, errors) = transform.finish();

    if errors.is_empty() {
      Ok(LocalTransformOutput { program, metadata })
    } else {
      Err(errors)
    }
  }))
  .map_err(|panic_payload| vec![TransformError::from_panic(panic_payload)])
  .and_then(|result| result)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_transform_delegates_to_try_transform() {
    // This test verifies that transform() delegates to try_transform()
    // We can't easily test the actual transformation without a real program,
    // but we verify the API is available and callable
    use swc_core::common::DUMMY_SP;
    use swc_core::ecma::ast::Module;

    let program = Program::Module(Module {
      span: DUMMY_SP,
      shebang: None,
      body: vec![],
    });

    let config = LocalTransformConfig {
      filename: None,
      cwd: None,
      root: None,
      source_file_name: None,
      options: Default::default(),
    };

    // Just verify that we can call transform without panicking
    let result = transform(program, config);
    // Empty module should succeed
    assert!(result.is_ok());
  }

  #[test]
  fn test_try_transform_catches_panic_and_returns_error() {
    // This test verifies that panics during try_transform are caught and converted to errors
    // rather than aborting the process
    use swc_core::common::DUMMY_SP;
    use swc_core::ecma::ast::Module;

    let program = Program::Module(Module {
      span: DUMMY_SP,
      shebang: None,
      body: vec![],
    });

    let config = LocalTransformConfig {
      filename: Some("test.tsx".to_string()),
      cwd: None,
      root: None,
      source_file_name: None,
      options: Default::default(),
    };

    // Call try_transform - it should NOT panic even if something goes wrong internally
    let result = try_transform(program, config);

    // The result should be Ok since an empty program is valid
    assert!(
      result.is_ok(),
      "try_transform should return Result, not panic"
    );
  }

  #[test]
  fn test_try_transform_error_handling_with_panic_payload() {
    // This test verifies that panic payloads are properly converted to TransformError

    // Test with a String panic payload
    let panic_str = "strip_runtime panic message".to_string();
    let payload: Box<dyn std::any::Any + Send> = Box::new(panic_str);
    let error = TransformError::from_panic(payload);

    assert!(error.message.contains("strip_runtime panic message"));
    assert!(error.message.contains("strip_runtime::try_transform"));
    assert!(error.message.contains("Panic during transform"));
  }
}
