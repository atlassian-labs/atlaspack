use swc_core::ecma::ast::Program;
use swc_core::ecma::visit::VisitMutWith;

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
