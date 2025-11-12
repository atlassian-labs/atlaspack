use std::fs;

mod support;

use serde_json::Value;
use support::{
  EnvGuard, canonicalize_output, emit_program, fixtures_dir, load_fixture_config, parse_program,
  run_transform,
};

#[test]
fn fixture_outputs_match() {
  let dir = fixtures_dir();
  let entries = fs::read_dir(&dir).expect("fixtures directory should exist");
  for entry in entries {
    let entry = entry.expect("failed to read fixture entry");
    if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
      continue;
    }
    let fixture_path = entry.path();
    if let Ok(filter) = std::env::var("FIXTURE_FILTER") {
      if fixture_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name != filter)
        .unwrap_or(true)
      {
        continue;
      }
    }
    let input_path = ["in.jsx", "in.tsx"]
      .into_iter()
      .map(|name| fixture_path.join(name))
      .find(|path| path.exists())
      .unwrap_or_else(|| {
        panic!(
          "fixture {:?} is missing required input file (expected in.jsx or in.tsx)",
          fixture_path
        )
      });
    let expected_path = fixture_path.join("out.js");
    if !expected_path.exists() {
      panic!("fixture {:?} is missing required file out.js", fixture_path);
    }
    let input = fs::read_to_string(&input_path).expect("failed to read fixture input");
    let expected_source =
      fs::read_to_string(&expected_path).expect("failed to read fixture output");
    let (expected_program, _) = parse_program(&expected_path, &expected_source);
    let expected = canonicalize_output(&emit_program(&expected_program));
    let (config_json, node_env, babel_env) = load_fixture_config(&fixture_path);
    let _guard = EnvGuard::new(node_env.as_deref(), babel_env.as_deref());
    let (actual_output, artifacts) = run_transform(&input_path, &input, &config_json);
    let actual = canonicalize_output(&actual_output);
    if let Ok(filter) = std::env::var("FIXTURE_DEBUG") {
      if fixture_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|name| name == filter)
        .unwrap_or(false)
      {
        println!("expected:\n{}", normalize(&expected));
        println!("actual:\n{}", normalize(&actual));
      }
    }
    assert_eq!(
      normalize(&expected),
      normalize(&actual),
      "fixture {:?} did not match",
      fixture_path.file_name().unwrap()
    );

    let expected_style_rules_path = fixture_path.join("style-rules.json");
    if expected_style_rules_path.exists() {
      let expected_style_rules = load_expected_style_rules(&expected_style_rules_path);
      let mut actual_rules = artifacts.style_rules.clone();
      let mut expected_rules = expected_style_rules;
      if let Some(style_filter) = std::env::var_os("FIXTURE_STYLE_DEBUG") {
        let style_filter = style_filter.to_string_lossy();
        let matches = fixture_path
          .file_name()
          .and_then(|n| n.to_str())
          .map(|name| style_filter.is_empty() || name == style_filter)
          .unwrap_or(false);
        if matches {
          println!(
            "style debug ({:?}) actual: {:?}",
            fixture_path.file_name().unwrap(),
            actual_rules
          );
          println!(
            "style debug ({:?}) expected: {:?}",
            fixture_path.file_name().unwrap(),
            expected_rules
          );
        }
      }
      actual_rules.sort();
      expected_rules.sort();
      assert_eq!(
        expected_rules,
        actual_rules,
        "fixture {:?} style rules did not match",
        fixture_path.file_name().unwrap()
      );
    }
  }
}

fn normalize(output: &str) -> String {
  output.replace("\r\n", "\n").trim().to_string()
}

fn load_expected_style_rules(path: &std::path::Path) -> Vec<String> {
  let raw = fs::read_to_string(path).expect("failed to read style-rules.json");
  let value: Value = serde_json::from_str(&raw).expect("failed to parse style-rules.json as JSON");
  match value {
    Value::Array(items) => items
      .into_iter()
      .map(|item| {
        item
          .as_str()
          .expect("expected style rule entries to be strings")
          .to_string()
      })
      .collect(),
    Value::Object(mut obj) => {
      let rules = obj
        .remove("styleRules")
        .expect("expected 'styleRules' property in style-rules.json object");
      rules
        .as_array()
        .expect("expected 'styleRules' to be an array")
        .iter()
        .map(|item| {
          item
            .as_str()
            .expect("expected style rule entries to be strings")
            .to_string()
        })
        .collect()
    }
    _ => panic!("style-rules.json must be an array or object with styleRules"),
  }
}
