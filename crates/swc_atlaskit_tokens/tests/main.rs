use std::panic;
use std::path::PathBuf;
use swc_atlaskit_tokens::{
  design_system_tokens_visitor,
  token_map::{TokenMap, load_token_map_from_json},
};

use swc_core::ecma::parser::{EsSyntax, Syntax};
use swc_core::ecma::transforms::testing::test_transform;
use swc_core::plugin::proxies::PluginCommentsProxy;

static TOKEN_DATA_PATH: &str = "tests/fixtures/token-data.json5";

// Helper function to run a transform test with a specific token map
fn run_transform_test(input: &str, expected: &str, token_map: Option<&TokenMap>) {
  test_transform(
    Syntax::Es(EsSyntax {
      jsx: true,
      ..Default::default()
    }),
    None,
    |_| {
      design_system_tokens_visitor(
        PluginCommentsProxy,
        true,
        false,
        vec![],
        "light".to_string(),
        false,
        token_map,
      )
    },
    input,
    expected,
  );
}

// Helper function to run a test with both None and loaded token maps
fn run_test_both_token_maps(test_name: &str, input: &str, expected: &str) {
  println!("Running {} with None token_map", test_name);
  run_transform_test(input, expected, None);

  println!("Running {} with loaded token_map", test_name);
  let loaded_token_map = load_token_map_from_json(&PathBuf::from(TOKEN_DATA_PATH))
    .expect("Failed to load token map for test");
  run_transform_test(input, expected, Some(&loaded_token_map));
}

// Helper function to run error tests with both token map scenarios
fn run_error_test_both_token_maps(test_name: &str, input: &str) {
  println!("Running {} with None token_map", test_name);
  let res = panic::catch_unwind(|| {
    run_transform_test(input, "", None);
  });
  assert!(res.is_err(), "Expected error with None token_map");

  println!("Running {} with loaded token_map", test_name);
  let loaded_token_map = load_token_map_from_json(&PathBuf::from(TOKEN_DATA_PATH))
    .expect("Failed to load token map for test");
  let res = panic::catch_unwind(|| {
    run_transform_test(input, "", Some(&loaded_token_map));
  });
  assert!(res.is_err(), "Expected error with loaded token_map");
}

#[test]
fn transforms_token_string_literal_with_auto_fallback_default() {
  let input = r#"
    import { token } from '@atlaskit/tokens';
    const v = token('color.text');
  "#;
  // Expect var(--ds-text, <light fallback>)
  let expected = r#"
    const v = "var(--ds-text, #172B4D)";
  "#;

  run_test_both_token_maps(
    "transforms_token_string_literal_with_auto_fallback_default",
    input,
    expected,
  );
}

#[test]
fn transforms_token_with_string_fallback() {
  let input = r#"
    import { token } from '@atlaskit/tokens';
    const v = token('color.text', 'red');
  "#;
  let expected = r#"
    const v = "var(--ds-text, red)";
  "#;

  run_test_both_token_maps("transforms_token_with_string_fallback", input, expected);
}

#[test]
fn transforms_token_with_expression_fallback_template_literal() {
  let input = r#"
    import { token } from '@atlaskit/tokens';
    const x = 'red';
    const v = token('color.text', x);
  "#;
  let expected = r#"
    const x = 'red';
    const v = `var(--ds-text, ${x})`;
  "#;

  run_test_both_token_maps(
    "transforms_token_with_expression_fallback_template_literal",
    input,
    expected,
  );
}

#[test]
fn errors_on_no_arguments() {
  let input = r#"
    import { token } from '@atlaskit/tokens';
    const v = token();
  "#;

  run_error_test_both_token_maps("errors_on_no_arguments", input);
}

#[test]
fn errors_on_non_string_first_arg() {
  let input = r#"
    import { token } from '@atlaskit/tokens';
    const name = 'color.text';
    const v = token(name);
  "#;

  run_error_test_both_token_maps("errors_on_non_string_first_arg", input);
}

#[test]
fn errors_on_too_many_args() {
  let input = r#"
    import { token } from '@atlaskit/tokens';
    const v = token('color.text', 'red', 'extra');
  "#;

  run_error_test_both_token_maps("errors_on_too_many_args", input);
}

#[test]
fn errors_on_unknown_token() {
  let input = r#"
    import { token } from '@atlaskit/tokens';
    const v = token('does.not.exist');
  "#;

  run_error_test_both_token_maps("errors_on_unknown_token", input);
}
