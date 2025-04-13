use atlaspack_swc_runner::runner::{run_visit, run_with_transformation};
use napi_derive::napi;
use swc_core::common::util::take::Take;
use swc_core::{atoms::Atom, ecma::ast::Program};
use swc_ecma_minifier::option::{ExtraOptions, MinifyOptions};

#[napi(object)]
pub struct JsOptimizerInput {
  pub code: String,
  pub source_maps: bool,
}

#[napi(object)]
pub struct JsOptimizerResult {
  pub code: String,
  pub source_map: Option<String>,
}

#[napi]
pub fn run_js_optimizer(input: JsOptimizerInput) -> napi::Result<JsOptimizerResult> {
  let result = run_with_transformation(&input.code, |context, module| {
    let program = swc_ecma_minifier::optimize(
      Program::Module(module.take()),
      context.source_map,
      None,
      None,
      &MinifyOptions {
        rename: true,
        compress: None,
        mangle: None,
        wrap: true,
        enclose: true,
      },
      &ExtraOptions {
        unresolved_mark: context.unresolved_mark,
        top_level_mark: context.global_mark,
        mangle_name_cache: None,
      },
    );

    module = match module {
      Program::Module(module) => Program::Module(module),
      Program::Script(script) => Program::Module(Module {
        span: script.span,
        shebang: None,
        body: script.body.into_iter().map(ModuleItem::Stmt).collect(),
      }),
    };
  })
  .map_err(|err| {
    napi::Error::from_reason(format!(
      "[napi] Failed to run inline require optimizer: {}",
      err
    ))
  })?;

  Ok(JsOptimizerResult {
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
