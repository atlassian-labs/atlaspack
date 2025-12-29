use swc_core::common::comments::Comment;
use swc_core::ecma::visit::VisitMutWith;
use swc_core::{common::comments::SingleThreadedComments, ecma::ast::Program};

pub use crate::babel_plugin::CompiledCssInJsTransform;
#[allow(unused_imports)]
pub use crate::types::{
  CacheBehavior, PluginOptions, ResolverOption, TransformFile, TransformMetadata, TransformOutput,
};

/// Entry point mirroring `packages/babel-plugin/src/index.ts`.
pub fn transform(program: Program, options: PluginOptions) -> TransformOutput {
  let mut transform = CompiledCssInJsTransform::new(options);
  let mut program = program;
  program.visit_mut_with(&mut transform);

  let metadata: TransformMetadata = transform.into_metadata();

  TransformOutput { program, metadata }
}

/// Entry point that allows callers to provide file metadata before running the transform.
pub fn transform_with_file(
  program: Program,
  file: TransformFile,
  options: PluginOptions,
) -> TransformOutput {
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
}

const DEFAULT_IMPORT_SOURCES: &[&str] = &["@compiled/react", "@atlaskit/css"];

pub fn should_run_compiled_css_in_js_transform(code: &str, options: PluginOptions) -> bool {
  let has_import_source = if let Some(import_sources) = options.import_sources {
    import_sources.iter().any(|source| {
      code.contains(source.as_str()) && !code.contains(&format!("{}/runtime", source))
    })
  } else {
    DEFAULT_IMPORT_SOURCES
      .iter()
      .any(|source| code.contains(source) && !code.contains(&format!("{}/runtime", source)))
  };

  has_import_source
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
