use std::fs;
use std::path::{Path, PathBuf};

use swc_common::comments::SingleThreadedComments;
use swc_common::errors::{HANDLER, Handler};
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::Program;
use swc_ecma_codegen::{Emitter, text_writer::JsWriter};
use swc_ecma_parser::{Parser, StringInput, Syntax};

fn transform_source(src: String, file_name: &str, syntax: Syntax) -> String {
  let cm: Lrc<SourceMap> = Default::default();
  let fm = cm.new_source_file(FileName::Custom(file_name.into()).into(), src);
  let comments = SingleThreadedComments::default();
  let mut parser = Parser::new(syntax, StringInput::from(&*fm), Some(&comments));
  let module = parser.parse_module().expect("parse module");
  let program = Program::Module(module);

  // Set pragma flag by scanning already-parsed comments (no extra source scan)
  // Scan both leading and trailing comments for the pragma without re-scanning source text.
  let (leading, trailing) = comments.take_all();
  let has_jsx_source_pragma = leading
    .borrow()
    .values()
    .flat_map(|v| v.iter())
    .any(|c| c.text.contains("@jsxImportSource @compiled/react"))
    || trailing
      .borrow()
      .values()
      .flat_map(|v| v.iter())
      .any(|c| c.text.contains("@jsxImportSource @compiled/react"));
  atlassian_swc_compiled_css::set_jsx_import_source_compiled(has_jsx_source_pragma);

  // Detect classic jsx pragma local name like /** @jsx jsx */ and set it
  let mut classic_local: Option<String> = None;
  for c in leading.borrow().values().flat_map(|v| v.iter()) {
    if let Some(idx) = c.text.find("@jsx ") {
      let rest = &c.text[idx + 5..];
      let name = rest.split_whitespace().next().unwrap_or("");
      if !name.is_empty() {
        classic_local = Some(name.to_string());
        break;
      }
    }
  }
  if classic_local.is_none() {
    for c in trailing.borrow().values().flat_map(|v| v.iter()) {
      if let Some(idx) = c.text.find("@jsx ") {
        let rest = &c.text[idx + 5..];
        let name = rest.split_whitespace().next().unwrap_or("");
        if !name.is_empty() {
          classic_local = Some(name.to_string());
          break;
        }
      }
    }
  }
  atlassian_swc_compiled_css::set_jsx_classic_pragma_local(classic_local);

  // Install a diagnostics handler so transform code can emit span-rich errors
  let handler = Handler::with_emitter_writer(Box::new(std::io::sink()), Some(cm.clone()));
  let out_program = HANDLER.set(&handler, || {
    atlassian_swc_compiled_css::process_transform(program)
  });

  let mut buf = Vec::new();
  {
    let mut emitter = Emitter {
      cfg: Default::default(),
      cm: cm.clone(),
      comments: None,
      wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
    };
    emitter.emit_program(&out_program).expect("emit program");
  }
  String::from_utf8(buf).expect("utf8")
}

fn reemit_source_without_transform(src: String, file_name: &str, syntax: Syntax) -> String {
  let cm: Lrc<SourceMap> = Default::default();
  let fm = cm.new_source_file(FileName::Custom(file_name.into()).into(), src);
  let mut parser = Parser::new(syntax, StringInput::from(&*fm), None);
  let module = parser.parse_module().expect("parse module (expected)");
  let program = Program::Module(module);

  let mut buf = Vec::new();
  {
    let mut emitter = Emitter {
      cfg: Default::default(),
      cm: cm.clone(),
      comments: None,
      wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
    };
    emitter
      .emit_program(&program)
      .expect("emit expected program");
  }
  String::from_utf8(buf).expect("utf8")
}

fn normalize_code_for_compare(code: &str) -> String {
  code
    .chars()
    .filter(|c| !c.is_whitespace() && *c != ';')
    .collect()
}

fn run_fixture_dir(case_dir: &Path) {
  let in_jsx_path = case_dir.join("in.jsx");
  let in_js_path = case_dir.join("in.js");
  let (in_path, is_jsx) = if in_jsx_path.exists() {
    (in_jsx_path, true)
  } else {
    (in_js_path, false)
  };
  let out_path = case_dir.join("out.js");
  if !in_path.exists() || !out_path.exists() {
    return;
  }
  let input = fs::read_to_string(&in_path).expect("read input file");
  let expected = fs::read_to_string(&out_path).expect("read out.js");
  let file_name = in_path
    .file_name()
    .and_then(|s| s.to_str())
    .unwrap_or(if is_jsx { "input.jsx" } else { "input.js" });
  let mut syntax = Syntax::Es(Default::default());
  if is_jsx {
    if let Syntax::Es(ref mut es_cfg) = syntax {
      es_cfg.jsx = true;
    }
  }
  let actual = transform_source(input, file_name, syntax.clone());
  // Normalize both actual and expected by parsing and re-emitting with the same emitter config
  let normalized_actual = reemit_source_without_transform(
    actual,
    if is_jsx { "actual.jsx" } else { "actual.js" },
    syntax.clone(),
  );
  let normalized_expected = reemit_source_without_transform(
    expected,
    if is_jsx {
      "expected.jsx"
    } else {
      "expected.js"
    },
    syntax,
  );
  let left = normalize_code_for_compare(normalized_actual.trim());
  let right = normalize_code_for_compare(normalized_expected.trim());
  assert_eq!(left, right, "fixture mismatch: {}", case_dir.display());
}

#[test]
fn fixtures_match_out_js() {
  let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("tests")
    .join("fixtures");
  let only = std::env::var("FIXTURE_ONLY").ok().filter(|s| !s.is_empty());
  for entry in fs::read_dir(&root).expect("read fixtures root") {
    let entry = entry.expect("dir entry");
    let case_dir = entry.path();
    if case_dir.is_dir() {
      if let Some(ref name) = only {
        let dir_name = case_dir.file_name().and_then(|s| s.to_str());
        if dir_name != Some(name.as_str()) {
          continue;
        }
      }
      run_fixture_dir(&case_dir);
    }
  }
}
