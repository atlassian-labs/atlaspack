//! Collect style rules by running the Atlassian design tokens transform
//! followed by the Compiled SWC plugin. Outputs JSON that matches the Babel
//! collector so we can diff results.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use once_cell::sync::Lazy;
use serde_json::{Value, json};
use swc_core::common::{
  FileName, GLOBALS, Globals, Mark, SourceMap, comments::SingleThreadedComments,
};
use swc_core::ecma::ast::{EsVersion, Pass, Program};
use swc_core::ecma::codegen::{Config as CodegenConfig, Emitter, Node, text_writer::JsWriter};
use swc_core::ecma::parser::{
  EsSyntax, Parser as SwcParser, StringInput, Syntax, TsSyntax, lexer::Lexer,
};
use swc_core::ecma::transforms::base::resolver;
use swc_core::ecma::visit::VisitMutWith;
use walkdir::WalkDir;

use compiled_swc_plugin::{EmitCommentsGuard, StyleArtifacts, take_latest_artifacts};
use swc_design_system_tokens::design_system_tokens_visitor;

const IMPORT_MARKERS: [&str; 2] = ["@compiled/react", "@atlaskit/css"];
const MAX_FILE_SIZE_BYTES: usize = 2 * 1024 * 1024;
const SUPPORTED_EXTENSIONS: [&str; 8] = ["ts", "tsx", "js", "jsx", "mjs", "cjs", "cts", "mts"];
const IGNORED_DIRECTORIES: [&str; 10] = [
  "node_modules",
  ".git",
  "dist",
  "build",
  ".next",
  "coverage",
  ".storybook",
  "storybook-static",
  ".yarn",
  ".turbo",
];

static CM: Lazy<Arc<SourceMap>> = Lazy::new(Default::default);

#[derive(Parser, Debug)]
struct Cli {
  #[clap(long, default_value = "../../../../../jira")]
  jira_root: String,

  #[clap(long, default_value = "../../../../../jira/tmp/style-rules/swc")]
  output_dir: String,

  #[clap(long)]
  config: Option<String>,

  #[clap(long)]
  limit: Option<usize>,

  #[clap(long)]
  only: Option<String>,
}

#[derive(Default)]
struct Stats {
  scanned: usize,
  matched: usize,
  transformed: usize,
  written: usize,
  skipped_large: usize,
  errors: usize,
}

#[derive(Default, Clone)]
struct TokensOptions {
  should_use_auto_fallback: bool,
  should_force_auto_fallback: bool,
  force_auto_fallback_exemptions: Vec<String>,
  default_theme: String,
}

fn main() -> Result<()> {
  let cli = Cli::parse();
  let jira_root = PathBuf::from(&cli.jira_root).canonicalize()?;
  let output_dir = PathBuf::from(&cli.output_dir);
  let config_path = cli
    .config
    .map(PathBuf::from)
    .unwrap_or_else(|| jira_root.join(".compiledcssrc"));

  if output_dir.exists() {
    fs::remove_dir_all(&output_dir)
      .with_context(|| format!("Failed to clear {}", output_dir.display()))?;
  }
  fs::create_dir_all(&output_dir)
    .with_context(|| format!("Failed to create {}", output_dir.display()))?;

  let config_raw = fs::read_to_string(&config_path).with_context(|| {
    format!(
      "Failed to read compiled config at {}",
      config_path.display()
    )
  })?;
  let config: Value = serde_json::from_str(&config_raw)
    .with_context(|| format!("Failed to parse JSON from {}", config_path.display()))?;

  let mut tokens_options = extract_tokens_options(&config);
  // Align with the Babel collector, which always enables automatic fallbacks and
  // forces default token fallbacks regardless of any project config overrides.
  tokens_options.should_use_auto_fallback = true;
  tokens_options.should_force_auto_fallback = true;

  let mut stats = Stats::default();
  let mut error_paths: Vec<PathBuf> = Vec::new();
  let mut files: Vec<PathBuf> = WalkDir::new(&jira_root)
    .follow_links(false)
    .into_iter()
    .filter_map(|entry| entry.ok())
    .filter(|entry| should_include(entry.path()))
    .filter(|entry| entry.file_type().is_file())
    .map(|entry| entry.path().to_path_buf())
    .collect();
  files.sort();

  if let Some(only) = &cli.only {
    let requested = PathBuf::from(only);
    let target = if requested.is_absolute() {
      requested
    } else {
      jira_root.join(requested)
    };
    let canonical_target = target.canonicalize().with_context(|| {
      format!(
        "Failed to resolve path for --only option at {}",
        target.display()
      )
    })?;
    if should_include(&canonical_target) {
      files = vec![canonical_target];
    } else {
      files.clear();
    }
  }

  if let Some(limit) = cli.limit {
    files.truncate(limit);
  }

  for file_path in files {
    stats.scanned += 1;

    let metadata = match fs::metadata(&file_path) {
      Ok(meta) => meta,
      Err(err) => {
        stats.errors += 1;
        eprintln!("Failed to stat {}: {err}", file_path.display());
        continue;
      }
    };
    if metadata.len() as usize > MAX_FILE_SIZE_BYTES {
      stats.skipped_large += 1;
      continue;
    }

    let source = match fs::read_to_string(&file_path) {
      Ok(src) => src,
      Err(err) => {
        stats.errors += 1;
        eprintln!("Failed to read {}: {err}", file_path.display());
        continue;
      }
    };

    if !IMPORT_MARKERS.iter().any(|marker| source.contains(marker)) {
      continue;
    }
    stats.matched += 1;

    match process_file(
      &file_path,
      &source,
      &jira_root,
      &output_dir,
      &tokens_options,
      &config_raw,
    ) {
      Ok(wrote_output) => {
        stats.transformed += 1;
        if wrote_output {
          stats.written += 1;
        }
      }
      Err(err) => {
        stats.errors += 1;
        eprintln!("Failed to transform {}: {err:#}", file_path.display());
        error_paths.push(file_path.clone());
      }
    }
  }

  println!("Scanned: {}", stats.scanned);
  println!("Matched: {}", stats.matched);
  println!("Transformed: {}", stats.transformed);
  println!("Written: {}", stats.written);
  println!("Skipped (too large): {}", stats.skipped_large);
  println!("Errors: {}", stats.errors);
  println!("Output directory: {}", output_dir.display());

  if !error_paths.is_empty() {
    let log_path = output_dir.join("errors.log");
    let log_contents = error_paths
      .iter()
      .map(|path| path.to_string_lossy())
      .collect::<Vec<_>>()
      .join("\n");

    match fs::write(&log_path, format!("{log_contents}\n")) {
      Ok(_) => {
        eprintln!(
          "{} files failed to transform. See {} for details.",
          error_paths.len(),
          log_path.display()
        );
      }
      Err(write_err) => {
        eprintln!(
          "Failed to write error log at {}: {write_err}",
          log_path.display()
        );
      }
    }
  }

  Ok(())
}

fn should_include(path: &Path) -> bool {
  if path.components().any(|component| {
    IGNORED_DIRECTORIES.contains(&component.as_os_str().to_string_lossy().as_ref())
  }) {
    return false;
  }

  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext))
    .unwrap_or(false)
}

fn process_file(
  path: &Path,
  source: &str,
  jira_root: &Path,
  output_dir: &Path,
  tokens_options: &TokensOptions,
  compiled_config_json: &str,
) -> Result<bool> {
  let rel_path = path.strip_prefix(jira_root).unwrap_or(path);

  let artifacts = run_pipeline(path, source, tokens_options, compiled_config_json)?;
  if artifacts.style_rules.is_empty() {
    return Ok(false);
  }

  let output_file = build_output_path(output_dir, rel_path);
  if let Some(parent) = output_file.parent() {
    fs::create_dir_all(parent).with_context(|| format!("Failed to create {}", parent.display()))?;
  }

  let mut style_rules = artifacts.style_rules;
  let aria_count = style_rules
    .iter()
    .filter(|rule| rule.contains("aria"))
    .count();
  if std::env::var_os("COMPILED_DEBUG_CSS").is_some() {
    eprintln!("[compiled-debug] style_rules aria count = {}", aria_count);
    for rule in style_rules.iter().take(10) {
      eprintln!("[compiled-debug] style_rule {}", rule);
    }
  }
  style_rules.sort();

  let payload = json!({
      "source": rel_path.to_string_lossy(),
      "styleRules": style_rules,
  });
  let json = serde_json::to_string_pretty(&payload)?;
  fs::write(&output_file, format!("{json}\n"))
    .with_context(|| format!("Failed to write {}", output_file.display()))?;

  Ok(true)
}

fn run_pipeline(
  path: &Path,
  source: &str,
  tokens_options: &TokensOptions,
  compiled_config_json: &str,
) -> Result<StyleArtifacts> {
  let cm = CM.clone();
  let filename = path.to_path_buf();

  GLOBALS.set(&Globals::new(), || {
    let fm = cm.new_source_file(FileName::Real(filename.clone()).into(), source.to_string());
    let comments = SingleThreadedComments::default();
    let _emitter_guard = EmitCommentsGuard::new(&comments);
    let syntax = compiled_syntax(path);
    let is_typescript = matches!(syntax, Syntax::Typescript(_));

    let lexer = Lexer::new(
      syntax.clone(),
      EsVersion::Es2022,
      StringInput::from(&*fm),
      Some(&comments),
    );
    let mut parser = SwcParser::new_from(lexer);
    let mut program = parser
      .parse_program()
      .map_err(|err| anyhow!("failed to parse {}: {:?}", path.display(), err))?;
    for err in parser.take_errors() {
      return Err(anyhow!("failed to parse {}: {:?}", path.display(), err));
    }

    let unresolved_mark = Mark::new();
    let top_level_mark = Mark::new();
    let mut resolver_pass = resolver(unresolved_mark, top_level_mark, is_typescript);
    program.visit_mut_with(&mut resolver_pass);

    let mut tokens_pass = design_system_tokens_visitor(
      comments.clone(),
      tokens_options.should_use_auto_fallback,
      tokens_options.should_force_auto_fallback,
      tokens_options.force_auto_fallback_exemptions.clone(),
      tokens_options.default_theme.clone(),
      false,
    );
    tokens_pass.process(&mut program);

    // Reset any previous artifacts collected on this thread.
    take_latest_artifacts();

    let filename_for_transform = filename.to_string_lossy().to_string();
    let transformed = compiled_swc_plugin::transform_program_for_testing(
      program,
      filename_for_transform,
      Some(compiled_config_json),
    );

    let artifacts = take_latest_artifacts();
    ensure_printable(&transformed)?;

    Ok(artifacts)
  })
}

fn ensure_printable(program: &Program) -> Result<()> {
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
    program.emit_with(&mut emitter)?;
  }
  // Drop the buffer; we only care that emission worked.
  Ok(())
}

fn compiled_syntax(path: &Path) -> Syntax {
  let is_ts = path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| {
      matches!(
        ext.to_ascii_lowercase().as_str(),
        "ts" | "tsx" | "cts" | "mts"
      )
    })
    .unwrap_or(false);

  if is_ts {
    Syntax::Typescript(TsSyntax {
      tsx: path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("tsx"))
        .unwrap_or(false),
      decorators: true,
      ..Default::default()
    })
  } else {
    Syntax::Es(EsSyntax {
      jsx: path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("jsx") || ext.eq_ignore_ascii_case("tsx"))
        .unwrap_or(false),
      decorators: true,
      export_default_from: true,
      import_attributes: true,
      ..Default::default()
    })
  }
}

fn extract_tokens_options(config: &Value) -> TokensOptions {
  let mut opts = TokensOptions {
    should_use_auto_fallback: true,
    should_force_auto_fallback: false,
    force_auto_fallback_exemptions: Vec::new(),
    default_theme: "light".to_string(),
  };

  if let Some(plugins) = config
    .get("transformerBabelPlugins")
    .and_then(|v| v.as_array())
  {
    for entry in plugins {
      match entry {
        Value::String(name) if name == "@atlaskit/tokens/babel-plugin" => {}
        Value::Array(items) if matches!(items.first(), Some(Value::String(name)) if name == "@atlaskit/tokens/babel-plugin") => {
          if let Some(Value::Object(options)) = items.get(1) {
            if let Some(value) = options
              .get("shouldUseAutoFallback")
              .and_then(|v| v.as_bool())
            {
              opts.should_use_auto_fallback = value;
            }
            if let Some(value) = options
              .get("shouldForceAutoFallback")
              .and_then(|v| v.as_bool())
            {
              opts.should_force_auto_fallback = value;
            }
            if let Some(array) = options
              .get("forceAutoFallbackExemptions")
              .and_then(|v| v.as_array())
            {
              opts.force_auto_fallback_exemptions = array
                .iter()
                .filter_map(|value| value.as_str().map(|s| s.to_string()))
                .collect();
            }
            if let Some(theme) = options.get("defaultTheme").and_then(|v| v.as_str()) {
              opts.default_theme = theme.to_string();
            }
          }
        }
        _ => {}
      }
    }
  }

  opts
}

fn build_output_path(output_dir: &Path, rel_path: &Path) -> PathBuf {
  let mut dest = output_dir.to_path_buf();
  if let Some(parent) = rel_path.parent() {
    dest.push(parent);
  }
  if let Some(file_name) = rel_path.file_name().and_then(|name| name.to_str()) {
    dest.push(format!("{file_name}-style-rules.json"));
  } else {
    dest.push("style-rules.json");
  }
  dest
}
