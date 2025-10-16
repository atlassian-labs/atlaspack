use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use swc_common::comments::SingleThreadedComments;
use swc_common::errors::{HANDLER, Handler};
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::Program;
use swc_ecma_codegen::{Emitter, text_writer::JsWriter};
use swc_ecma_parser::{Parser, StringInput, Syntax};

fn transform_source(
  src: String,
  file_name: &str,
  syntax: Syntax,
  config: atlassian_swc_compiled_css::config::CompiledCssInJsTransformConfig,
) -> String {
  let cm: Lrc<SourceMap> = Default::default();
  let fm = cm.new_source_file(FileName::Custom(file_name.into()).into(), src);
  let comments = SingleThreadedComments::default();
  let mut parser = Parser::new(syntax, StringInput::from(&*fm), Some(&comments));
  let module = parser.parse_module().expect("parse module");
  let mut program = Program::Module(module);

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
  HANDLER.set(&handler, || {
    atlassian_swc_compiled_css::apply_compiled_atomic_with_config(&mut program, config)
  });

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
  String::from_utf8(buf).expect("utf8")
}

fn process_fixture_dir(
  case_dir: &Path,
  force_overwrite: bool,
) -> Result<(), Box<dyn std::error::Error>> {
  let in_jsx_path = case_dir.join("in.jsx");
  let in_js_path = case_dir.join("in.js");
  let (in_path, is_jsx) = if in_jsx_path.exists() {
    (in_jsx_path, true)
  } else if in_js_path.exists() {
    (in_js_path, false)
  } else {
    println!("Skipping {}: no input file found", case_dir.display());
    return Ok(());
  };

  let input = fs::read_to_string(&in_path)?;
  let file_name = in_path
    .file_name()
    .and_then(|s| s.to_str())
    .unwrap_or(if is_jsx { "input.jsx" } else { "input.js" });

  let mut syntax = Syntax::Es(Default::default());
  if is_jsx && let Syntax::Es(ref mut es_cfg) = syntax {
    es_cfg.jsx = true;
  }

  println!("Processing {}", case_dir.display());
  for extract in [true, false] {
    let out_path = case_dir.join(if extract { "extract.js" } else { "out.js" });

    // Check if out.js already exists and we're not forcing overwrite
    if out_path.exists() && !force_overwrite {
      println!(
        "Skipping {}: out.js already exists (use --force to overwrite)",
        case_dir.display()
      );
      return Ok(());
    }
    let transformed = transform_source(
      input.clone(),
      file_name,
      syntax,
      atlassian_swc_compiled_css::config::CompiledCssInJsTransformConfig {
        extract: Some(extract),
        ..Default::default()
      },
    );
    fs::write(&out_path, transformed)?;
    println!("Generated {}", out_path.display());
  }

  Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args: Vec<String> = env::args().collect();
  let mut force_overwrite = false;
  let mut target_fixture: Option<String> = None;

  // Parse command line arguments
  let mut i = 1;
  while i < args.len() {
    match args[i].as_str() {
      "--force" | "-f" => {
        force_overwrite = true;
      }
      "--fixture" => {
        if i + 1 < args.len() {
          target_fixture = Some(args[i + 1].clone());
          i += 1;
        } else {
          eprintln!("Error: --fixture requires a fixture name");
          std::process::exit(1);
        }
      }
      "--help" | "-h" => {
        println!("Usage: {} [OPTIONS]", args[0]);
        println!("Options:");
        println!("  --force, -f         Overwrite existing out.js files");
        println!("  --fixture <name>    Process only the specified fixture");
        println!("  --help, -h          Show this help message");
        std::process::exit(0);
      }
      _ => {
        eprintln!("Unknown argument: {}", args[i]);
        eprintln!("Use --help for usage information");
        std::process::exit(1);
      }
    }
    i += 1;
  }

  let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("tests")
    .join("fixtures");

  if !root.exists() {
    eprintln!("Error: fixtures directory not found at {}", root.display());
    std::process::exit(1);
  }

  let mut processed_count = 0;
  let mut error_count = 0;

  for entry in fs::read_dir(&root)? {
    let entry = entry?;
    let case_dir = entry.path();

    if !case_dir.is_dir() {
      continue;
    }

    // If a specific fixture is requested, skip others
    if let Some(ref target) = target_fixture {
      let dir_name = case_dir.file_name().and_then(|s| s.to_str());
      if dir_name != Some(target.as_str()) {
        continue;
      }
    }

    match process_fixture_dir(&case_dir, force_overwrite) {
      Ok(()) => processed_count += 1,
      Err(e) => {
        eprintln!("Error processing {}: {}", case_dir.display(), e);
        error_count += 1;
      }
    }
  }

  if let Some(ref target) = target_fixture
    && processed_count == 0
    && error_count == 0
  {
    eprintln!("Error: fixture '{}' not found", target);
    std::process::exit(1);
  }

  println!("\nSummary:");
  println!("  Processed: {}", processed_count);
  if error_count > 0 {
    println!("  Errors: {}", error_count);
  }

  if error_count > 0 {
    std::process::exit(1);
  }

  Ok(())
}
