use atlaspack_swc_runner::runner::{run_visit, run_with_transformation};
use napi_derive::napi;
use swc_core::common::util::take::Take;
use swc_core::ecma::visit::VisitMutWith;
use swc_core::{atoms::Atom, ecma::ast::Program};
use swc_ecma_minifier::option::{CompressOptions, ExtraOptions, MangleOptions, MinifyOptions};

#[napi(object)]
pub struct JsOptimizerInput {
  pub code: String,
  pub source_maps: bool,
  pub inline_requires: Option<InlineRequireOptions>,
}

#[napi(object)]
pub struct InlineRequireOptions {
  ignore_module_ids: Vec<String>,
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
        compress: Some(CompressOptions::default()),
        mangle: Some(MangleOptions::default()),
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

    if let Some(InlineRequireOptions { ignore_module_ids }) = input.inline_requires {
      let visitor = atlaspack_plugin_optimizer_inline_requires::InlineRequiresOptimizer::builder()
        .unresolved_mark(context.unresolved_mark)
        .add_ignore_pattern(IgnorePattern::ModuleIdHashSet(
          input.ignore_module_ids.into_iter().map(Atom::new).collect(),
        ))
        .build();

      module.visit_mut_with(&mut visitor);
    }

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
  });
}
