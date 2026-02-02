use glob::glob;
use std::path::Path;
use std::process::Command;

fn main() {
  // Register all non-test TypeScript files in prelude/src for change detection
  for path in glob("prelude/src/**/*.ts")
    .expect("Failed to read glob pattern")
    .flatten()
  {
    // Skip test files
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str())
      && !file_name.ends_with(".test.ts")
    {
      println!("cargo:rerun-if-changed={}", path.display());
    }
  }

  // Run yarn build:lib in the prelude directory
  let prelude_dir = Path::new("prelude");
  if prelude_dir.exists() {
    let status = Command::new("yarn")
      .arg("build:lib")
      .current_dir(prelude_dir)
      .status()
      .expect("Failed to execute yarn build:lib");

    if !status.success() {
      panic!("yarn build:lib failed with exit code: {:?}", status.code());
    }
  }
}
