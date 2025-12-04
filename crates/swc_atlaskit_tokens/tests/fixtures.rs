use std::path::PathBuf;

use swc_atlaskit_tokens::{
  design_system_tokens_visitor,
  token_map::{TokenMap, load_token_map_from_json},
};

use swc_core::ecma::transforms::testing::{FixtureTestConfig, test_fixture};
use swc_core::plugin::proxies::PluginCommentsProxy;

static TOKEN_DATA_PATH: &str = "tests/fixtures/token-data.json5";

/// Generic test runner that can test with different token_map configurations
fn run_fixture_test(
  input_path: &str,
  should_use_auto_fallback: bool,
  should_force_auto_fallback: bool,
  exemptions: Vec<String>,
  default_theme: String,
  is_node_modules: bool,
  token_map: Option<&TokenMap>,
) {
  let input = PathBuf::from(input_path);
  let output = input.parent().unwrap().join("output.js");

  let exemptions_clone = exemptions.clone();
  let default_theme_clone = default_theme.clone();

  test_fixture(
    Default::default(),
    &|_| {
      design_system_tokens_visitor(
        PluginCommentsProxy,
        should_use_auto_fallback,
        should_force_auto_fallback,
        exemptions_clone.clone(),
        default_theme_clone.clone(),
        is_node_modules,
        token_map,
      )
    },
    &input,
    &output,
    FixtureTestConfig {
      ..Default::default()
    },
  );
}

#[testing::fixture("tests/fixtures/basic/input.js")]
fn fixture_basic_no_token_map(input: PathBuf) {
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "light".to_string(),
    false,
    None,
  );
}

#[testing::fixture("tests/fixtures/basic/input.js")]
fn fixture_basic_with_token_map(input: PathBuf) {
  let token_map = load_token_map_from_json(&PathBuf::from(TOKEN_DATA_PATH))
    .expect("Failed to load token map for fixture test");
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "light".to_string(),
    false,
    Some(&token_map),
  );
}

#[testing::fixture("tests/fixtures/no-auto-fallback/input.js")]
fn fixture_no_auto_fallback_no_token_map(input: PathBuf) {
  run_fixture_test(
    input.to_str().unwrap(),
    false,
    false,
    vec![],
    "light".to_string(),
    false,
    None,
  );
}

#[testing::fixture("tests/fixtures/no-auto-fallback/input.js")]
fn fixture_no_auto_fallback_with_token_map(input: PathBuf) {
  let token_map = load_token_map_from_json(&PathBuf::from(TOKEN_DATA_PATH))
    .expect("Failed to load token map for fixture test");
  run_fixture_test(
    input.to_str().unwrap(),
    false,
    false,
    vec![],
    "light".to_string(),
    false,
    Some(&token_map),
  );
}

#[testing::fixture("tests/fixtures/empty-string-fallback/input.js")]
fn fixture_empty_string_fallback_no_token_map(input: PathBuf) {
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "light".to_string(),
    false,
    None,
  );
}

#[testing::fixture("tests/fixtures/empty-string-fallback/input.js")]
fn fixture_empty_string_fallback_with_token_map(input: PathBuf) {
  let token_map = load_token_map_from_json(&PathBuf::from(TOKEN_DATA_PATH))
    .expect("Failed to load token map for fixture test");
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "light".to_string(),
    false,
    Some(&token_map),
  );
}

#[testing::fixture("tests/fixtures/force-auto-fallback/input.js")]
fn fixture_force_auto_fallback_no_token_map(input: PathBuf) {
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    true,
    vec![],
    "light".to_string(),
    false,
    None,
  );
}

#[testing::fixture("tests/fixtures/force-auto-fallback/input.js")]
fn fixture_force_auto_fallback_with_token_map(input: PathBuf) {
  let token_map = load_token_map_from_json(&PathBuf::from(TOKEN_DATA_PATH))
    .expect("Failed to load token map for fixture test");
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    true,
    vec![],
    "light".to_string(),
    false,
    Some(&token_map),
  );
}

#[testing::fixture("tests/fixtures/force-auto-exemptions/input.js")]
fn fixture_force_auto_exemptions_no_token_map(input: PathBuf) {
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    true,
    vec!["custom.exempt".to_string()],
    "light".to_string(),
    false,
    None,
  );
}

#[testing::fixture("tests/fixtures/force-auto-exemptions/input.js")]
fn fixture_force_auto_exemptions_with_token_map(input: PathBuf) {
  let token_map = load_token_map_from_json(&PathBuf::from(TOKEN_DATA_PATH))
    .expect("Failed to load token map for fixture test");
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    true,
    vec!["custom.exempt".to_string()],
    "light".to_string(),
    false,
    Some(&token_map),
  );
}

#[testing::fixture("tests/fixtures/legacy-light/input.js")]
fn fixture_legacy_light_no_token_map(input: PathBuf) {
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "legacy-light".to_string(),
    false,
    None,
  );
}

#[testing::fixture("tests/fixtures/legacy-light/input.js")]
fn fixture_legacy_light_with_token_map(input: PathBuf) {
  let token_map = load_token_map_from_json(&PathBuf::from(TOKEN_DATA_PATH))
    .expect("Failed to load token map for fixture test");
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "legacy-light".to_string(),
    false,
    Some(&token_map),
  );
}

#[testing::fixture("tests/fixtures/alias-import/input.js")]
fn fixture_alias_import_no_token_map(input: PathBuf) {
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "light".to_string(),
    false,
    None,
  );
}

#[testing::fixture("tests/fixtures/alias-import/input.js")]
fn fixture_alias_import_with_token_map(input: PathBuf) {
  let token_map = load_token_map_from_json(&PathBuf::from(TOKEN_DATA_PATH))
    .expect("Failed to load token map for fixture test");
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "light".to_string(),
    false,
    Some(&token_map),
  );
}

#[testing::fixture("tests/fixtures/remove-entire-import/input.js")]
fn fixture_remove_entire_import_no_token_map(input: PathBuf) {
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "light".to_string(),
    false,
    None,
  );
}

#[testing::fixture("tests/fixtures/remove-entire-import/input.js")]
fn fixture_remove_entire_import_with_token_map(input: PathBuf) {
  let token_map = load_token_map_from_json(&PathBuf::from(TOKEN_DATA_PATH))
    .expect("Failed to load token map for fixture test");
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "light".to_string(),
    false,
    Some(&token_map),
  );
}

#[testing::fixture("tests/fixtures/force-auto-expression/input.js")]
fn fixture_force_auto_expression_no_token_map(input: PathBuf) {
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    true,
    vec![],
    "light".to_string(),
    false,
    None,
  );
}

#[testing::fixture("tests/fixtures/force-auto-expression/input.js")]
fn fixture_force_auto_expression_with_token_map(input: PathBuf) {
  let token_map = load_token_map_from_json(&PathBuf::from(TOKEN_DATA_PATH))
    .expect("Failed to load token map for fixture test");
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    true,
    vec![],
    "light".to_string(),
    false,
    Some(&token_map),
  );
}

#[testing::fixture("tests/fixtures/local-token-function/input.js")]
fn fixture_local_token_function_no_token_map(input: PathBuf) {
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "light".to_string(),
    false,
    None,
  );
}

#[testing::fixture("tests/fixtures/local-token-function/input.js")]
fn fixture_local_token_function_with_token_map(input: PathBuf) {
  let token_map = load_token_map_from_json(&PathBuf::from(TOKEN_DATA_PATH))
    .expect("Failed to load token map for fixture test");
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "light".to_string(),
    false,
    Some(&token_map),
  );
}

#[testing::fixture("tests/fixtures/foreign-source-token/input.js")]
fn fixture_foreign_source_token_no_token_map(input: PathBuf) {
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "light".to_string(),
    false,
    None,
  );
}

#[testing::fixture("tests/fixtures/foreign-source-token/input.js")]
fn fixture_foreign_source_token_with_token_map(input: PathBuf) {
  let token_map = load_token_map_from_json(&PathBuf::from(TOKEN_DATA_PATH))
    .expect("Failed to load token map for fixture test");
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "light".to_string(),
    false,
    Some(&token_map),
  );
}

#[testing::fixture("tests/fixtures/custom-fallback-radius/input.js")]
fn fixture_custom_fallback_radius_no_token_map(input: PathBuf) {
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "light".to_string(),
    false,
    None,
  );
}

#[testing::fixture("tests/fixtures/custom-fallback-radius/input.js")]
fn fixture_custom_fallback_radius_with_token_map(input: PathBuf) {
  let token_map = load_token_map_from_json(&PathBuf::from(TOKEN_DATA_PATH))
    .expect("Failed to load token map for fixture test");
  run_fixture_test(
    input.to_str().unwrap(),
    true,
    false,
    vec![],
    "light".to_string(),
    false,
    Some(&token_map),
  );
}
