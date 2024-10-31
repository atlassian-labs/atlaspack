use atlaspack_plugin_optimizer_inline_requires::IgnorePattern;
use atlaspack_swc_runner::runner::run_visit;
use napi_derive::napi;
use swc_core::atoms::Atom;

#[napi(object)]
pub struct InlineRequiresOptimizerInput {
  pub code: String,
  pub source_maps: bool,
  pub ignore_module_ids: Vec<String>,
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
  let result = run_visit(&input.code, |ctx| {
    atlaspack_plugin_optimizer_inline_requires::InlineRequiresOptimizer::builder()
      .unresolved_mark(ctx.unresolved_mark)
      .add_ignore_pattern(IgnorePattern::ModuleIdHashSet(
        input.ignore_module_ids.into_iter().map(Atom::new).collect(),
      ))
      .build()
  })
  .map_err(|err| {
    napi::Error::from_reason(format!(
      "[napi] Failed to run inline require optimizer: {}",
      err
    ))
  })?;

  Ok(InlineRequiresOptimizerResult {
    code: result.output_code,
    source_map: if input.source_maps {
      let source_map = String::from_utf8(result.source_map).map_err(|err| {
        napi::Error::from_reason(format!("[napi] Invalid utf-8 source map output: {}", err))
      })?;
      Some(source_map)
    } else {
      None
    },
  })
}
