use atlaspack_plugin_optimizer_inline_requires::IgnorePattern;
use atlaspack_swc_runner::runner::{
  RunContext, RunWithTransformationOptions, RunWithTransformationOutput, run_with_transformation,
};
use napi::{Env, JsObject};
use napi_derive::napi;
use swc_core::atoms::Atom;
use swc_core::ecma::ast::Module;
use swc_core::ecma::visit::VisitMutWith;

#[napi(object)]
pub struct InlineRequiresOptimizerInput {
  pub code: String,
  pub source_maps: bool,
  pub ignore_module_ids: Vec<String>,
  /// Optional JSON-serialized source map describing where `code` came from.
  /// When supplied, the returned source map already resolves through to the
  /// original sources, avoiding a downstream `<anon>` + `extends()`
  /// composition (which mis-attributes tokens near asset boundaries).
  pub input_source_map: Option<String>,
}

#[napi(object)]
pub struct InlineRequiresOptimizerResult {
  pub code: String,
  pub source_map: Option<String>,
}

#[napi]
pub fn run_inline_requires_optimizer(
  input: InlineRequiresOptimizerInput,
) -> napi::Result<InlineRequiresOptimizerResult> {
  let ignore_module_ids = input.ignore_module_ids;
  let input_source_map = input
    .input_source_map
    .as_deref()
    .map(|raw| swc_sourcemap::SourceMap::from_slice(raw.as_bytes()))
    .transpose()
    .map_err(|err| {
      napi::Error::from_reason(format!(
        "[napi] Invalid input source map JSON for inline-requires: {}",
        err
      ))
    })?;

  let options = RunWithTransformationOptions {
    code: &input.code,
    input_source_map,
    ..RunWithTransformationOptions::default()
  };

  let RunWithTransformationOutput {
    output_code,
    source_map,
    ..
  } = run_with_transformation(options, |ctx: RunContext, module: &mut Module| {
    let mut visit = atlaspack_plugin_optimizer_inline_requires::InlineRequiresOptimizer::builder()
      .unresolved_mark(ctx.unresolved_mark)
      .add_ignore_pattern(IgnorePattern::ModuleIdHashSet(
        ignore_module_ids.into_iter().map(Atom::new).collect(),
      ))
      .build();
    module.visit_mut_with(&mut visit);
  })
  .map_err(|err| {
    napi::Error::from_reason(format!(
      "[napi] Failed to run inline require optimizer: {}",
      err
    ))
  })?;

  Ok(InlineRequiresOptimizerResult {
    code: output_code,
    source_map: if input.source_maps {
      let source_map = String::from_utf8(source_map).map_err(|err| {
        napi::Error::from_reason(format!("[napi] Invalid utf-8 source map output: {}", err))
      })?;
      Some(source_map)
    } else {
      None
    },
  })
}

/// Runs in the rayon thread pool
#[napi]
pub fn run_inline_requires_optimizer_async(
  env: Env,
  input: InlineRequiresOptimizerInput,
) -> napi::Result<JsObject> {
  let (deferred, promise) = env.create_deferred()?;

  rayon::spawn(move || {
    let result = run_inline_requires_optimizer(input);
    match result {
      Ok(result) => {
        deferred.resolve(move |_env| Ok(result));
      }
      Err(err) => {
        deferred.reject(napi::Error::new(
          napi::Status::GenericFailure,
          format!("[napi] Failed to run inline require optimizer: {}", err),
        ));
      }
    }
  });

  Ok(promise)
}
