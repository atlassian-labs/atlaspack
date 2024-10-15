use crate::{Config, SourceType, TransformResult};

/// Parse a file with the `atlaspack_js_swc_core` parser for testing
pub fn run_swc_core_transform(source: &str) -> TransformResult {
  let swc_output = crate::transform(make_test_swc_config(source), None).unwrap();
  swc_output
}

/// SWC configuration for testing
pub fn make_test_swc_config(source: &str) -> Config {
  Config {
    source_type: SourceType::Module,
    is_browser: true,
    filename: "something/file.js".to_string(),
    inline_fs: true,
    code: source.as_bytes().to_vec(),
    scope_hoist: true,
    ..Default::default()
  }
}
