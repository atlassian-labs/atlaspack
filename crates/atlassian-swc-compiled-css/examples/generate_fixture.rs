use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser;

#[path = "../tests/support/mod.rs"]
mod support;

#[derive(Parser, Debug)]
#[command(
  name = "generate_fixture",
  about = "Generate actual.js for a compiled fixture"
)]
struct Args {
  /// Name of the fixture folder inside tests/fixtures
  fixture: String,
}

fn main() -> Result<()> {
  let args = Args::parse();
  let fixtures_root = support::fixtures_dir();
  let fixture_dir = fixtures_root.join(&args.fixture);
  ensure_fixture_exists(&fixture_dir)?;

  let input_path = ["in.jsx", "in.tsx"]
    .into_iter()
    .map(|name| fixture_dir.join(name))
    .find(|path| path.exists())
    .ok_or_else(|| anyhow::anyhow!("fixture input (in.jsx or in.tsx) not found"))?;
  let input = fs::read_to_string(&input_path)
    .with_context(|| format!("failed to read fixture input {}", input_path.display()))?;

  let (config_json, node_env, babel_env) = support::load_fixture_config(&fixture_dir);
  let _guard = support::EnvGuard::new(node_env.as_deref(), babel_env.as_deref());

  let (output_raw, _) = support::run_transform(&input_path, &input, &config_json);
  let output = support::canonicalize_output(&output_raw);
  let actual_path = fixture_dir.join("actual.js");
  fs::write(&actual_path, format!("{}\n", output))
    .with_context(|| format!("failed to write {}", actual_path.display()))?;

  println!("Wrote {}", actual_path.display());

  Ok(())
}

fn ensure_fixture_exists(path: &Path) -> Result<()> {
  if !path.exists() {
    anyhow::bail!(
      "fixture '{}' was not found in {}",
      path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<unknown>"),
      support::fixtures_dir().display()
    );
  }
  if !path.is_dir() {
    anyhow::bail!("fixture path {} is not a directory", path.display());
  }
  Ok(())
}
