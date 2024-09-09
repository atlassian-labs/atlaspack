use atlaspack_js_swc_core::runner::run_visit;
use atlaspack_plugin_optimizer_inline_requires::IgnorePattern;
use napi_derive::napi;
use swc_core::atoms::Atom;

#[napi(object)]
pub struct InlineRequiresOptimizerInput {
  pub input_code: String,
  pub source_maps: bool,
  pub assets_to_ignore: Vec<String>,
}

#[napi(object)]
pub struct InlineRequiresOptimizerResult {
  pub output_code: String,
  pub source_map: Option<String>,
}

#[napi]
pub fn run_inline_requires_optimizer(
  input: InlineRequiresOptimizerInput,
) -> napi::Result<InlineRequiresOptimizerResult> {
  let result = run_visit(&input.input_code, |ctx| {
    let visitor = atlaspack_plugin_optimizer_inline_requires::InlineRequiresOptimizer::builder()
      .unresolved_mark(ctx.unresolved_mark)
      .add_ignore_pattern(IgnorePattern::ModuleIdHashSet(
        input
          .assets_to_ignore
          .into_iter()
          .map(|s| Atom::new(s))
          .collect(),
      ))
      .build();
    visitor
  })
  .map_err(|err| napi::Error::from_reason(format!("[napi] Failed to run inline require optimizer: {}", err)))?;

  Ok(InlineRequiresOptimizerResult {
    output_code: result.output_code,
    source_map: if input.source_maps {
      let source_map = String::from_utf8(result.source_map).map_err(|err| {
        napi::Error::from_reason(format!("[napi] Non-utf8 source-map output: {}", err))
      })?;
      Some(source_map)
    } else {
      None
    },
  })
}
