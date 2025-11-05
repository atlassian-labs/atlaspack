use compiled_swc_plugin::{
  EmitCommentsGuard, StyleArtifacts, take_latest_artifacts, transform_program_for_testing,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use swc_common::comments::SingleThreadedComments;
use swc_common::sync::Lrc;
use swc_core::common::{FileName, GLOBALS, Globals, Mark, SourceMap};
use swc_core::ecma::ast::{EsVersion, Pass, Program};
use swc_core::ecma::codegen::{Config as CodegenConfig, Emitter, text_writer::JsWriter};
use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax, TsSyntax, lexer::Lexer};
use swc_ecma_transforms_base::resolver;
use swc_ecma_transforms_react::{Options as ReactOptions, Runtime, react};

fn syntax_for_filename(path: &Path) -> Syntax {
  let name = path.to_string_lossy();
  if name.ends_with(".ts") || name.ends_with(".tsx") || name.ends_with(".cts") {
    Syntax::Typescript(TsSyntax {
      tsx: name.ends_with(".tsx"),
      decorators: true,
      ..Default::default()
    })
  } else {
    Syntax::Es(EsSyntax {
      jsx: name.ends_with(".jsx") || name.ends_with(".tsx"),
      decorators: true,
      export_default_from: true,
      import_attributes: true,
      ..Default::default()
    })
  }
}

pub fn parse_program(path: &Path, source: &str) -> (Program, SingleThreadedComments) {
  use std::sync::Arc;

  let cm: Arc<SourceMap> = Default::default();
  let filename = FileName::Real(path.to_path_buf());
  let fm = cm.new_source_file(filename.into(), source.into());
  let comments = SingleThreadedComments::default();
  let lexer = Lexer::new(
    syntax_for_filename(path),
    EsVersion::Es2022,
    StringInput::from(&*fm),
    Some(&comments),
  );
  let mut parser = Parser::new_from(lexer);
  (
    Program::Module(parser.parse_module().expect("failed to parse module")),
    comments,
  )
}

pub fn emit_program(program: &Program) -> String {
  use std::sync::Arc;

  let cm: Arc<SourceMap> = Default::default();
  let mut buf = Vec::new();
  {
    let writer = JsWriter::new(cm.clone(), "\n", &mut buf, None);
    let mut cfg = CodegenConfig::default();
    cfg.target = EsVersion::Es2022;
    let mut emitter = Emitter {
      cfg,
      comments: None,
      cm,
      wr: writer,
    };
    emitter
      .emit_program(program)
      .expect("failed to emit program");
  }
  String::from_utf8(buf).expect("emitted JS should be utf8")
}

pub fn run_transform(
  input_path: &Path,
  source: &str,
  config_json: &str,
) -> (String, StyleArtifacts) {
  GLOBALS.set(&Globals::new(), || {
    let (program, comments) = parse_program(input_path, source);
    let _emitter_guard = EmitCommentsGuard::new(&comments);
    let mut transformed = transform_program_for_testing(
      program,
      input_path.to_string_lossy().to_string(),
      Some(config_json),
    )
    .expect("transform program");
    {
      let cm: Lrc<SourceMap> = Default::default();
      let top_level_mark = Mark::fresh(Mark::root());
      let unresolved_mark = Mark::fresh(Mark::root());
      {
        let mut pass = resolver(unresolved_mark, top_level_mark, false);
        pass.process(&mut transformed);
      }
      {
        let mut react_options = ReactOptions::default();
        react_options.runtime = Some(Runtime::Automatic);
        react_options.development = Some(false);
        let mut pass = react(
          cm,
          None::<SingleThreadedComments>,
          react_options,
          top_level_mark,
          unresolved_mark,
        );
        pass.process(&mut transformed);
      }
    }
    let output = emit_program(&transformed);
    let artifacts = take_latest_artifacts();
    (output, artifacts)
  })
}

pub struct EnvGuard {
  prev_node: Option<String>,
  prev_babel: Option<String>,
}

impl EnvGuard {
  pub fn new(node_env: Option<&str>, babel_env: Option<&str>) -> Self {
    let prev_node = std::env::var("NODE_ENV").ok();
    let prev_babel = std::env::var("BABEL_ENV").ok();

    match node_env {
      Some(value) => unsafe { std::env::set_var("NODE_ENV", value) },
      None => unsafe { std::env::remove_var("NODE_ENV") },
    }

    match babel_env {
      Some(value) => unsafe { std::env::set_var("BABEL_ENV", value) },
      None => unsafe { std::env::remove_var("BABEL_ENV") },
    }

    EnvGuard {
      prev_node,
      prev_babel,
    }
  }
}

impl Drop for EnvGuard {
  fn drop(&mut self) {
    match &self.prev_node {
      Some(value) => unsafe { std::env::set_var("NODE_ENV", value) },
      None => unsafe { std::env::remove_var("NODE_ENV") },
    }
    match &self.prev_babel {
      Some(value) => unsafe { std::env::set_var("BABEL_ENV", value) },
      None => unsafe { std::env::remove_var("BABEL_ENV") },
    }
  }
}

pub fn load_fixture_config(path: &Path) -> (String, Option<String>, Option<String>) {
  let config_path = path.join("config.json");
  if !config_path.exists() {
    return (String::from("{\"extract\":false}"), None, None);
  }

  let raw = fs::read_to_string(&config_path).expect("failed to read config.json");
  let mut value: Value = serde_json::from_str(&raw).expect("failed to parse config.json");

  let node_env = value
    .get("nodeEnv")
    .and_then(|v| v.as_str())
    .map(|s| s.to_string());
  let babel_env = value
    .get("babelEnv")
    .and_then(|v| v.as_str())
    .map(|s| s.to_string());

  if let Some(obj) = value.as_object_mut() {
    obj.remove("nodeEnv");
    obj.remove("babelEnv");
    obj.entry("extract").or_insert(Value::Bool(false));
  }

  let config_json = serde_json::to_string(&value).expect("failed to serialize config");
  (config_json, node_env, babel_env)
}

pub fn fixtures_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("tests")
    .join("fixtures")
}

pub fn canonicalize_output(output: &str) -> String {
  let mut react_import: Option<&str> = None;
  let mut other_imports: Vec<&str> = Vec::new();
  let mut body: Vec<&str> = Vec::new();

  for line in output.lines() {
    if line.starts_with("import ") {
      if line.contains("* as React") {
        react_import = Some(line);
      } else {
        other_imports.push(line);
      }
    } else {
      body.push(line);
    }
  }

  let mut runtime_imports: Vec<&str> = Vec::new();
  let mut jsx_runtime_imports: Vec<&str> = Vec::new();
  let mut remaining_imports: Vec<&str> = Vec::new();
  for line in other_imports {
    if line.contains("@compiled/react/runtime") {
      runtime_imports.push(line);
    } else if line.contains("react/jsx-runtime") {
      jsx_runtime_imports.push(line);
    } else {
      remaining_imports.push(line);
    }
  }

  runtime_imports.sort();
  remaining_imports.sort();
  jsx_runtime_imports.sort();

  let mut result = String::new();
  if let Some(line) = react_import {
    result.push_str(line);
    result.push('\n');
  }
  for line in runtime_imports {
    result.push_str(line);
    result.push('\n');
  }
  for line in remaining_imports {
    result.push_str(line);
    result.push('\n');
  }
  for line in jsx_runtime_imports {
    result.push_str(line);
    result.push('\n');
  }
  for (index, line) in body.iter().enumerate() {
    result.push_str(line);
    if index + 1 < body.len() {
      result.push('\n');
    }
  }

  result = result
    .replace(
      "import { jsx as _jsx, jsxs as _jsxs } from \"react/jsx-runtime\";\n",
      "import { jsx, jsxs } from \"react/jsx-runtime\";\n",
    )
    .replace(
      "import { jsx as _jsx, jsxs } from \"react/jsx-runtime\";\n",
      "import { jsx, jsxs } from \"react/jsx-runtime\";\n",
    )
    .replace(
      "import { jsx as _jsx } from \"react/jsx-runtime\";\n",
      "import { jsx } from \"react/jsx-runtime\";\n",
    )
    .replace(
      "import { jsxs as _jsxs } from \"react/jsx-runtime\";\n",
      "import { jsxs } from \"react/jsx-runtime\";\n",
    )
    .replace("_jsx(", "jsx(")
    .replace("_jsxs(", "jsxs(")
    .replace(
      "import * as React from \"react\";",
      "import * as React from 'react';",
    );

  let mut seen_jsx_import = false;
  let filtered_lines: Vec<&str> = result
    .lines()
    .filter(|line| {
      let trimmed = line.trim();
      if trimmed == "import { jsx } from \"react/jsx-runtime\";" {
        if seen_jsx_import {
          return false;
        }
        seen_jsx_import = true;
      }
      true
    })
    .collect();
  result = filtered_lines.join("\n");

  if result.contains("import { jsx, jsxs } from \"react/jsx-runtime\";") {
    let filtered: Vec<&str> = result
      .lines()
      .filter(|line| line.trim() != "import { jsx } from \"react/jsx-runtime\";")
      .collect();
    result = filtered.join("\n");
  }

  if std::env::var_os("COMPILED_DEBUG_CANON").is_some() {
    eprintln!("[canon]\\n{}", result);
  }

  result
}
