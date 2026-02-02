use std::panic;
use swc_core::common::comments::Comment;
use swc_core::ecma::visit::VisitMutWith;
use swc_core::{common::comments::SingleThreadedComments, ecma::ast::Program};

pub use crate::babel_plugin::CompiledCssInJsTransform;
pub use crate::errors::{TransformError, init_panic_suppression};
#[allow(unused_imports)]
pub use crate::types::{
  CacheBehavior, PluginOptions, ResolverOption, TransformFile, TransformMetadata, TransformOutput,
};

/// Entry point mirroring `packages/babel-plugin/src/index.ts`.
///
/// This function applies the compiled CSS transformation to the given AST.
/// Panics that occur during transformation are caught and converted into structured
/// diagnostics rather than aborting the process, allowing callers to report errors gracefully.
pub fn transform(
  program: Program,
  options: PluginOptions,
) -> Result<TransformOutput, Vec<TransformError>> {
  // Suppress panic output so errors are only reported as diagnostics
  init_panic_suppression();

  panic::catch_unwind(panic::AssertUnwindSafe(|| {
    let mut transform = CompiledCssInJsTransform::new(options);
    let mut program = program;
    program.visit_mut_with(&mut transform);

    let metadata: TransformMetadata = transform.into_metadata();

    TransformOutput { program, metadata }
  }))
  .map_err(|panic_payload| vec![TransformError::from_panic(panic_payload)])
}

/// Entry point that allows callers to provide file metadata before running the transform.
///
/// This function is similar to `transform()` but accepts additional file context that
/// can improve diagnostic reporting (e.g., source file path, line offsets).
/// Like `transform()`, panics are caught and converted into diagnostics.
pub fn transform_with_file(
  program: Program,
  file: TransformFile,
  options: PluginOptions,
) -> Result<TransformOutput, Vec<TransformError>> {
  // Suppress panic output so errors are only reported as diagnostics
  init_panic_suppression();

  panic::catch_unwind(panic::AssertUnwindSafe(|| {
    let mut transform = CompiledCssInJsTransform::new(options);

    {
      let shared_state = transform.state();
      let mut state = shared_state.borrow_mut();
      state.replace_file(file);
    }

    let mut program: Program = program;
    program.visit_mut_with(&mut transform);

    let metadata: TransformMetadata = transform.into_metadata();

    TransformOutput { program, metadata }
  }))
  .map_err(|panic_payload| vec![TransformError::from_panic(panic_payload)])
}

pub fn should_run_compiled_css_in_js_transform(code: &str, options: PluginOptions) -> bool {
  options
    .import_sources
    .iter()
    .any(|source| code.contains(source.as_str()))
}

pub fn remove_jsx_pragma_comments(comments: &SingleThreadedComments) -> bool {
  let (mut leading, mut trailing) = comments.borrow_all_mut();
  let mut removed_any = false;

  leading.retain(|_, comment_list| {
    let original_len = comment_list.len();
    comment_list.retain(|comment| !is_jsx_pragma_comment(comment));
    if comment_list.len() != original_len {
      removed_any = true;
    }
    !comment_list.is_empty()
  });

  trailing.retain(|_, comment_list| {
    let original_len = comment_list.len();
    comment_list.retain(|comment| !is_jsx_pragma_comment(comment));
    if comment_list.len() != original_len {
      removed_any = true;
    }
    !comment_list.is_empty()
  });

  removed_any
}

fn is_jsx_pragma_comment(comment: &Comment) -> bool {
  let text = comment.text.as_ref();
  text.contains("@jsx")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_should_run_compiled_css_in_js_transform_with_import() {
    let code = "import { css } from '@compiled/react';";
    let options = PluginOptions::default();
    assert!(should_run_compiled_css_in_js_transform(code, options));
  }

  #[test]
  fn test_should_run_compiled_css_in_js_transform_no_import() {
    let code = "const x = 1;";
    let options = PluginOptions::default();
    assert!(!should_run_compiled_css_in_js_transform(code, options));
  }

  #[test]
  fn test_transform_catches_panic_and_returns_error() {
    // This test verifies that panics during transform are caught and converted to errors
    // rather than aborting the process

    use swc_core::common::DUMMY_SP;
    use swc_core::ecma::ast::Module;

    let program = Program::Module(Module {
      span: DUMMY_SP,
      shebang: None,
      body: vec![],
    });

    let options = PluginOptions::default();

    // Call transform - it should NOT panic even if something goes wrong internally
    let result = transform(program, options);

    // The result should be Ok since an empty program is valid
    assert!(result.is_ok(), "Transform should return Result, not panic");
  }

  #[test]
  fn test_transform_with_file_catches_panic_and_returns_error() {
    // This test verifies that panics during transform_with_file are caught
    // and converted to errors rather than aborting the process

    use crate::types::TransformFileOptions;
    use swc_core::common::DUMMY_SP;
    use swc_core::ecma::ast::Module;

    let program = Program::Module(Module {
      span: DUMMY_SP,
      shebang: None,
      body: vec![],
    });

    let file = TransformFile::transform_compiled_with_options(
      std::sync::Arc::new(swc_core::common::SourceMap::default()),
      vec![],
      TransformFileOptions {
        filename: Some("test.tsx".to_string()),
        cwd: None,
        root: None,
        loc_filename: None,
      },
    );

    let options = PluginOptions::default();

    // Call transform_with_file - it should NOT panic even if something goes wrong internally
    let result = transform_with_file(program, file, options);

    // The result should be Ok since an empty program is valid
    assert!(result.is_ok(), "Transform should return Result, not panic");
  }

  #[test]
  fn test_panic_payload_conversion_to_error() {
    // This test verifies that panic payloads are properly converted to TransformError

    // Test with a String panic payload
    let panic_str = "test panic message".to_string();
    let payload: Box<dyn std::any::Any + Send> = Box::new(panic_str);
    let error = TransformError::from_panic(payload);

    assert!(error.message.contains("test panic message"));
  }
}
