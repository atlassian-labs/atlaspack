use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

fn main() {
  setup_atlaspack_rust_version();
}

fn setup_atlaspack_rust_version() {
  // Read the @atlaspack/rust package.json to get the version
  let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
  let workspace_root = Path::new(&cargo_manifest_dir)
    .parent()
    .unwrap()
    .parent()
    .unwrap();

  let rust_package_json_path = workspace_root
    .join("packages")
    .join("core")
    .join("rust")
    .join("package.json");

  let package_json_content = fs::read_to_string(&rust_package_json_path)
    .expect("Failed to read @atlaspack/rust package.json");

  // Parse the version from package.json
  let version = extract_version_from_package_json(&package_json_content)
    .expect("Failed to extract version from package.json");

  // Hash the version to create a u64 for cache keys
  let mut hasher = DefaultHasher::new();
  version.hash(&mut hasher);
  let version_hash = hasher.finish();

  // Set the environment variables that will be available during compilation
  println!("cargo:rustc-env=ATLASPACK_RUST_VERSION={}", version);
  println!(
    "cargo:rustc-env=ATLASPACK_RUST_VERSION_HASH={}",
    version_hash
  );

  // Tell cargo to rerun this script if the package.json changes
  println!(
    "cargo:rerun-if-changed={}",
    rust_package_json_path.display()
  );
}

fn extract_version_from_package_json(content: &str) -> Option<String> {
  // Simple regex-free parsing for the version field
  for line in content.lines() {
    let trimmed = line.trim();
    if trimmed.starts_with("\"version\":") {
      // Extract version from: "version": "3.13.0",
      // Skip past "version": and find the first quote
      if let Some(colon_pos) = trimmed.find(':') {
        let after_colon = &trimmed[colon_pos + 1..];
        if let Some(start) = after_colon.find('"')
          && let Some(end) = after_colon[start + 1..].find('"')
        {
          return Some(after_colon[start + 1..start + 1 + end].to_string());
        }
      }
    }
  }
  None
}
