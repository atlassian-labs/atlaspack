use crate::{
  CompiledCssInJsTransformResult, apply_compiled_atomic_with_config,
  config::CompiledCssInJsTransformConfig,
};
use swc_common::{FileName, SourceMap};
use swc_ecma_ast::Program;
use swc_ecma_codegen::{Emitter, text_writer::JsWriter};
use swc_ecma_parser::{Parser, StringInput, Syntax};

use swc_common::sync::Lrc;

pub fn transform_code(
  code: &str,
  file_name: &str,
  config: Option<CompiledCssInJsTransformConfig>,
) -> (String, CompiledCssInJsTransformResult) {
  // Parse the code using SWC parser
  let cm: Lrc<SourceMap> = Default::default();
  let fm = cm.new_source_file(FileName::Custom(file_name.into()).into(), code.to_string());
  let mut syntax: Syntax = Syntax::Es(Default::default());
  if file_name.ends_with(".jsx") {
    if let Syntax::Es(ref mut es_cfg) = syntax {
      es_cfg.jsx = true;
    }
  }
  let mut parser = Parser::new(syntax, StringInput::from(&*fm), None);
  let module = parser.parse_module().expect("parse module");
  let mut program = Program::Module(module);

  let result = apply_compiled_atomic_with_config(&mut program, config.unwrap_or_default());

  // Convert the transformed program back to string
  let mut buf = Vec::new();
  {
    let mut emitter = Emitter {
      cfg: Default::default(),
      cm: cm.clone(),
      comments: None,
      wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
    };
    emitter.emit_program(&program).expect("emit program");
  }
  (String::from_utf8(buf).expect("utf8"), result)
}

pub fn assert_contains(actual_code: &str, expected_code: &str) {
  assert!(
    actual_code.contains(expected_code),
    "Actual code does not contain expected code: {} {}",
    expected_code,
    actual_code
  );
}
