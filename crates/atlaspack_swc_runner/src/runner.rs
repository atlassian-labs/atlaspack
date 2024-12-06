use std::string::FromUtf8Error;
use swc_core::common::input::StringInput;
use swc_core::common::sync::Lrc;
use swc_core::common::util::take::Take;
use swc_core::common::{FileName, Globals, Mark, SourceMap, GLOBALS};
use swc_core::ecma::ast::Module;
use swc_core::ecma::codegen::text_writer::JsWriter;
use swc_core::ecma::parser::lexer::Lexer;
use swc_core::ecma::parser::Parser;
use swc_core::ecma::transforms::base::resolver;
use swc_core::ecma::visit::{Fold, FoldWith, Visit, VisitMut, VisitMutWith, VisitWith};

pub struct RunContext {
  /// Source-map in use
  pub source_map: Lrc<SourceMap>,
  /// Global mark from SWC resolver
  pub global_mark: Mark,
  /// Unresolved mark from SWC resolver
  pub unresolved_mark: Mark,
}

pub struct RunVisitResult<V> {
  pub output_code: String,
  #[allow(unused)]
  pub visitor: V,
  pub source_map: Vec<u8>,
}

/// Runner of SWC transformations
///
/// * Parse `code` with SWC
/// * Run a visitor over it
/// * Return the result
///
pub fn run_visit<V: VisitMut>(
  code: &str,
  make_visit: impl FnOnce(RunContext) -> V,
) -> Result<RunVisitResult<V>, RunWithTransformationError> {
  let (output_code, visitor, source_map) =
    run_with_transformation(code, |run_test_context: RunContext, module: &mut Module| {
      let mut visit = make_visit(run_test_context);
      module.visit_mut_with(&mut visit);
      visit
    })?;
  Ok(RunVisitResult {
    output_code,
    visitor,
    source_map,
  })
}

/// Same as `run_visit` but for `Visit` instead of `VisitMut`
pub fn run_visit_const<V: Visit>(
  code: &str,
  make_visit: impl FnOnce(RunContext) -> V,
) -> Result<RunVisitResult<V>, RunWithTransformationError> {
  let (output_code, visitor, source_map) =
    run_with_transformation(code, |run_test_context: RunContext, module: &mut Module| {
      let mut visit = make_visit(run_test_context);
      module.visit_with(&mut visit);
      visit
    })?;
  Ok(RunVisitResult {
    output_code,
    visitor,
    source_map,
  })
}

/// Same as `run_visit` but for `Fold` instances
pub fn run_fold<V: Fold>(
  code: &str,
  make_fold: impl FnOnce(RunContext) -> V,
) -> Result<RunVisitResult<V>, RunWithTransformationError> {
  let (output_code, visitor, source_map) =
    run_with_transformation(code, |run_test_context: RunContext, module: &mut Module| {
      let mut visit = make_fold(run_test_context);
      *module = module.take().fold_with(&mut visit);
      visit
    })?;
  Ok(RunVisitResult {
    output_code,
    visitor,
    source_map,
  })
}

#[derive(Debug, thiserror::Error)]
pub enum RunWithTransformationError {
  #[error("Failed to parse module")]
  SwcParse(swc_core::ecma::parser::error::Error),
  #[error("IO Error: {0}")]
  IoError(#[from] std::io::Error),
  #[error("Invalid utf-8 output: {0}")]
  InvalidUtf8Output(#[from] FromUtf8Error),
  #[error("Failed to generate source map")]
  SourceMap(#[from] sourcemap::Error),
}

type RunWithTransformationOutput<R> = (String, R, Vec<u8>);

/// Parse code, run resolver over it, then run the `tranform` function with the parsed module
/// codegen and return the results.
fn run_with_transformation<R>(
  code: &str,
  transform: impl FnOnce(RunContext, &mut Module) -> R,
) -> Result<RunWithTransformationOutput<R>, RunWithTransformationError> {
  let source_map = Lrc::new(SourceMap::default());
  let source_file = source_map.new_source_file(Lrc::new(FileName::Anon), code.into());

  let lexer = Lexer::new(
    Default::default(),
    Default::default(),
    StringInput::from(&*source_file),
    None,
  );

  let mut parser = Parser::new_from(lexer);
  let mut module = parser
    .parse_module()
    .map_err(RunWithTransformationError::SwcParse)?;

  GLOBALS.set(
    &Globals::new(),
    || -> Result<RunWithTransformationOutput<R>, RunWithTransformationError> {
      let global_mark = Mark::new();
      let unresolved_mark = Mark::new();
      module.visit_mut_with(&mut resolver(unresolved_mark, global_mark, false));

      let context = RunContext {
        source_map: source_map.clone(),
        global_mark,
        unresolved_mark,
      };
      let result = transform(context, &mut module);

      let mut line_pos_buffer = vec![];
      let mut output_buffer = vec![];
      let writer = JsWriter::new(
        source_map.clone(),
        "\n",
        &mut output_buffer,
        Some(&mut line_pos_buffer),
      );
      let mut emitter = swc_core::ecma::codegen::Emitter {
        cfg: Default::default(),
        cm: source_map.clone(),
        comments: None,
        wr: writer,
      };
      emitter.emit_module(&module)?;
      let output_code = String::from_utf8(output_buffer)?;
      let source_map = source_map.build_source_map(&line_pos_buffer);
      let mut output_map_buffer = vec![];
      source_map.to_writer(&mut output_map_buffer)?;

      Ok((output_code, result, output_map_buffer))
    },
  )
}

#[cfg(test)]
mod tests {
  use swc_core::ecma::ast::{Lit, Str};
  use swc_core::ecma::visit::VisitMut;

  use super::*;

  #[test]
  fn test_example() {
    struct Visitor;
    impl VisitMut for Visitor {
      fn visit_mut_lit(&mut self, n: &mut Lit) {
        *n = Lit::Str(Str::from("replacement"));
      }
    }

    let code = r#"console.log('test!')"#;
    let RunVisitResult { output_code, .. } = run_visit(code, |_: RunContext| Visitor).unwrap();
    assert_eq!(
      output_code,
      r#"console.log("replacement");
"#
    );
  }

  #[test]
  fn test_fold() {
    struct Folder;
    impl Fold for Folder {
      fn fold_lit(&mut self, _n: Lit) -> Lit {
        Lit::Str(Str::from("replacement"))
      }
    }

    let code = r#"console.log('test!')"#;
    let RunVisitResult { output_code, .. } = run_fold(code, |_: RunContext| Folder).unwrap();
    assert_eq!(
      output_code,
      r#"console.log("replacement");
"#
    );
  }
}
