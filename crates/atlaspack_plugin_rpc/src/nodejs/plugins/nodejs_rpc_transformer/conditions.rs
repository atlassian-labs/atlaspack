use atlaspack_core::types::Asset;
use regex::RegexSet;
use serde::Deserialize;

#[derive(Debug, Deserialize, Hash, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SerializableTransformerConditions {
  file_match: Option<Vec<String>>,
  code_match: Option<Vec<String>>,
  enabled: Option<bool>,
  origin: Option<OriginCondition>,
}

#[derive(Debug, Deserialize, Hash, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum OriginCondition {
  Source,
  ThirdParty,
}

#[derive(Debug)]
pub struct Conditions {
  file_match: Option<RegexSet>,
  code_match: Option<RegexSet>,
  enabled: bool,
  origin: Option<OriginCondition>,
}

impl Default for Conditions {
  fn default() -> Self {
    Self {
      file_match: None,
      code_match: None,
      enabled: true,
      origin: None,
    }
  }
}

impl TryFrom<Option<SerializableTransformerConditions>> for Conditions {
  type Error = anyhow::Error;

  fn try_from(value: Option<SerializableTransformerConditions>) -> anyhow::Result<Self> {
    let Some(value) = value else {
      return Ok(Conditions::default());
    };

    let code_match = if let Some(patterns) = value.code_match {
      Some(RegexSet::new(patterns)?)
    } else {
      None
    };

    let file_match = if let Some(pattern) = value.file_match {
      Some(RegexSet::new(&pattern)?)
    } else {
      None
    };

    Ok(Conditions {
      file_match,
      code_match,
      enabled: value.enabled.unwrap_or(true),
      origin: value.origin,
    })
  }
}

impl Conditions {
  pub fn should_skip(&self, asset: &Asset) -> anyhow::Result<bool> {
    if !self.enabled {
      return Ok(true);
    }

    if let Some(origin) = &self.origin {
      match origin {
        OriginCondition::Source => {
          if !asset.is_source {
            return Ok(true);
          }
        }
        OriginCondition::ThirdParty => {
          if asset.is_source {
            return Ok(true);
          }
        }
      }
    }

    if let Some(file_match) = &self.file_match
      && !file_match.is_match(
        asset
          .file_path
          .to_str()
          .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in file path"))?,
      )
    {
      return Ok(true);
    }

    if let Some(code_match) = &self.code_match {
      return Ok(!code_match.is_match(asset.code.as_str()?));
    }

    Ok(false)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use atlaspack_core::types::{Code, Environment};
  use pretty_assertions::assert_eq;
  use std::path::PathBuf;
  use std::sync::Arc;

  fn create_test_asset(code: &str, is_source: bool) -> Asset {
    Asset {
      code: Code::from(code),
      is_source,
      env: Arc::new(Environment::default()),
      file_path: PathBuf::from("test.js"),
      ..Asset::default()
    }
  }

  fn create_test_asset_with_path(code: &str, is_source: bool, file_path: &str) -> Asset {
    Asset {
      code: Code::from(code),
      is_source,
      env: Arc::new(Environment::default()),
      file_path: PathBuf::from(file_path),
      ..Asset::default()
    }
  }

  #[test]
  fn test_serializable_transformer_conditions_deserialization() {
    // Test basic deserialization with explicit enabled
    let json = r#"{"codeMatch": ["test"], "enabled": true}"#;
    let conditions: SerializableTransformerConditions = serde_json::from_str(json).unwrap();
    let expected = SerializableTransformerConditions {
      file_match: None,
      code_match: Some(vec!["test".to_string()]),
      enabled: Some(true),
      origin: None,
    };
    assert_eq!(conditions, expected);

    // Test with enabled omitted (should deserialize as None)
    let json = r#"{"codeMatch": ["test"]}"#;
    let conditions: SerializableTransformerConditions = serde_json::from_str(json).unwrap();
    let expected = SerializableTransformerConditions {
      file_match: None,
      code_match: Some(vec!["test".to_string()]),
      enabled: None,
      origin: None,
    };
    assert_eq!(conditions, expected);

    // Test with origin condition and explicit false
    let json = r#"{"enabled": false, "origin": "source"}"#;
    let conditions: SerializableTransformerConditions = serde_json::from_str(json).unwrap();
    let expected = SerializableTransformerConditions {
      file_match: None,
      code_match: None,
      enabled: Some(false),
      origin: Some(OriginCondition::Source),
    };
    assert_eq!(conditions, expected);

    // Test with third-party origin and explicit true
    let json = r#"{"enabled": true, "origin": "third-party"}"#;
    let conditions: SerializableTransformerConditions = serde_json::from_str(json).unwrap();
    let expected = SerializableTransformerConditions {
      file_match: None,
      code_match: None,
      enabled: Some(true),
      origin: Some(OriginCondition::ThirdParty),
    };
    assert_eq!(conditions, expected);
  }

  #[test]
  fn test_conditions_try_from_some() {
    let serializable = SerializableTransformerConditions {
      file_match: None,
      code_match: Some(vec!["console\\.log".to_string(), "alert".to_string()]),
      enabled: Some(false),
      origin: Some(OriginCondition::Source),
    };

    let conditions: Conditions = Some(serializable).try_into().unwrap();
    assert_eq!(conditions.code_match.is_some(), true);
    assert_eq!(conditions.enabled, false);
    assert_eq!(conditions.origin, Some(OriginCondition::Source));

    // Test that regex compilation works
    let regex_set = conditions.code_match.as_ref().unwrap();
    assert!(regex_set.is_match("console.log"));
    assert!(regex_set.is_match("alert"));
    assert!(!regex_set.is_match("warning"));
  }

  #[test]
  fn test_conditions_default() {
    let conditions = Conditions::default();
    assert_eq!(conditions.code_match.is_none(), true);
    assert_eq!(conditions.enabled, true);
    assert_eq!(conditions.origin, None);
  }

  #[test]
  fn test_conditions_try_from_none() {
    let conditions: Conditions = None.try_into().unwrap();
    assert_eq!(conditions.code_match.is_none(), true);
    assert_eq!(conditions.enabled, true);
    assert_eq!(conditions.origin, None);
  }

  #[test]
  fn test_conditions_try_from_enabled_defaults_to_true() {
    // Test that enabled defaults to true when None
    let serializable = SerializableTransformerConditions {
      file_match: None,
      code_match: None,
      enabled: None,
      origin: None,
    };

    let conditions: Conditions = Some(serializable).try_into().unwrap();
    assert_eq!(conditions.code_match.is_none(), true);
    assert_eq!(conditions.enabled, true); // Should default to true
    assert_eq!(conditions.origin, None);

    // Test with explicit false
    let serializable = SerializableTransformerConditions {
      file_match: None,
      code_match: None,
      enabled: Some(false),
      origin: None,
    };

    let conditions: Conditions = Some(serializable).try_into().unwrap();
    assert_eq!(conditions.enabled, false);

    // Test with explicit true
    let serializable = SerializableTransformerConditions {
      file_match: None,
      code_match: None,
      enabled: Some(true),
      origin: None,
    };

    let conditions: Conditions = Some(serializable).try_into().unwrap();
    assert_eq!(conditions.enabled, true);
  }

  #[test]
  fn test_conditions_try_from_invalid_regex() {
    let serializable = SerializableTransformerConditions {
      file_match: None,
      code_match: Some(vec!["[".to_string()]), // Invalid regex
      enabled: Some(true),
      origin: None,
    };

    let result: Result<Conditions, _> = Some(serializable).try_into();
    assert!(result.is_err());
  }

  #[test]
  fn test_should_skip_when_disabled() {
    let conditions = Conditions {
      file_match: None,
      code_match: None,
      enabled: false,
      origin: None,
    };

    let asset = create_test_asset("console.log('test')", true);
    assert!(conditions.should_skip(&asset).unwrap());
  }

  #[test]
  fn test_should_skip_origin_source() {
    let conditions = Conditions {
      file_match: None,
      code_match: None,
      enabled: true,
      origin: Some(OriginCondition::Source),
    };

    // Should not skip source assets
    let source_asset = create_test_asset("console.log('test')", true);
    assert!(!conditions.should_skip(&source_asset).unwrap());

    // Should skip third-party assets
    let third_party_asset = create_test_asset("console.log('test')", false);
    assert!(conditions.should_skip(&third_party_asset).unwrap());
  }

  #[test]
  fn test_should_skip_origin_third_party() {
    let conditions = Conditions {
      file_match: None,
      code_match: None,
      enabled: true,
      origin: Some(OriginCondition::ThirdParty),
    };

    // Should skip source assets
    let source_asset = create_test_asset("console.log('test')", true);
    assert!(conditions.should_skip(&source_asset).unwrap());

    // Should not skip third-party assets
    let third_party_asset = create_test_asset("console.log('test')", false);
    assert!(!conditions.should_skip(&third_party_asset).unwrap());
  }

  #[test]
  fn test_should_skip_code_match() {
    let regex_set = RegexSet::new(["console\\.log", "alert"]).unwrap();
    let conditions = Conditions {
      file_match: None,
      code_match: Some(regex_set),
      enabled: true,
      origin: None,
    };

    // Should not skip when code matches
    let matching_asset = create_test_asset("console.log('test')", true);
    assert!(!conditions.should_skip(&matching_asset).unwrap());

    let matching_asset2 = create_test_asset("alert('warning')", true);
    assert!(!conditions.should_skip(&matching_asset2).unwrap());

    // Should skip when code doesn't match
    let non_matching_asset = create_test_asset("const x = 5;", true);
    assert!(conditions.should_skip(&non_matching_asset).unwrap());
  }

  #[test]
  fn test_should_skip_complex_conditions() {
    let regex_set = RegexSet::new(["import.*react"]).unwrap();
    let conditions = Conditions {
      file_match: None,
      code_match: Some(regex_set),
      enabled: true,
      origin: Some(OriginCondition::Source),
    };

    // Source asset with matching code - should not skip
    let source_matching = create_test_asset("import React from 'react';", true);
    assert!(!conditions.should_skip(&source_matching).unwrap());

    // Third-party asset with matching code - should skip due to origin
    let third_party_matching = create_test_asset("import React from 'react';", false);
    assert!(conditions.should_skip(&third_party_matching).unwrap());

    // Source asset with non-matching code - should skip due to code
    let source_non_matching = create_test_asset("const x = 5;", true);
    assert!(conditions.should_skip(&source_non_matching).unwrap());

    // Third-party asset with non-matching code - should skip due to both
    let third_party_non_matching = create_test_asset("const x = 5;", false);
    assert!(conditions.should_skip(&third_party_non_matching).unwrap());
  }

  #[test]
  fn test_should_skip_no_conditions() {
    let conditions = Conditions {
      file_match: None,
      code_match: None,
      enabled: true,
      origin: None,
    };

    // When no conditions are set, should never skip
    let asset = create_test_asset("any code", true);
    assert!(!conditions.should_skip(&asset).unwrap());

    let asset2 = create_test_asset("any code", false);
    assert!(!conditions.should_skip(&asset2).unwrap());
  }

  #[test]
  fn test_should_skip_with_invalid_utf8() {
    let conditions = Conditions {
      file_match: None,
      code_match: Some(RegexSet::new(&["test"]).unwrap()),
      enabled: true,
      origin: None,
    };

    let mut asset = create_test_asset("test", true);
    // Insert invalid UTF-8 bytes
    asset.code = Code::new(vec![0xFF, 0xFE]);

    // Should return an error when trying to convert to string
    let result = conditions.should_skip(&asset);
    assert!(result.is_err());
  }

  #[test]
  fn test_file_match_deserialization() {
    // Test basic file_match deserialization
    let json = r#"{"fileMatch": "\\.ts$", "enabled": true}"#;
    let conditions: SerializableTransformerConditions = serde_json::from_str(json).unwrap();
    let expected = SerializableTransformerConditions {
      file_match: Some("\\.ts$".to_string()),
      code_match: None,
      enabled: Some(true),
      origin: None,
    };
    assert_eq!(conditions, expected);

    // Test file_match with other conditions
    let json = r#"{"fileMatch": "src/.*\\.js$", "codeMatch": ["console"], "origin": "source"}"#;
    let conditions: SerializableTransformerConditions = serde_json::from_str(json).unwrap();
    let expected = SerializableTransformerConditions {
      file_match: Some("src/.*\\.js$".to_string()),
      code_match: Some(vec!["console".to_string()]),
      enabled: None,
      origin: Some(OriginCondition::Source),
    };
    assert_eq!(conditions, expected);
  }

  #[test]
  fn test_file_match_try_from() {
    let serializable = SerializableTransformerConditions {
      file_match: Some("\\.tsx?$".to_string()),
      code_match: None,
      enabled: Some(true),
      origin: None,
    };

    let conditions: Conditions = Some(serializable).try_into().unwrap();
    assert!(conditions.file_match.is_some());
    assert_eq!(conditions.enabled, true);

    // Test that regex compilation works
    let file_regex = conditions.file_match.as_ref().unwrap();
    assert!(file_regex.is_match("app.ts"));
    assert!(file_regex.is_match("component.tsx"));
    assert!(!file_regex.is_match("styles.css"));
    assert!(!file_regex.is_match("app.js"));
  }

  #[test]
  fn test_file_match_try_from_invalid_regex() {
    let serializable = SerializableTransformerConditions {
      file_match: Some("[".to_string()), // Invalid regex
      code_match: None,
      enabled: Some(true),
      origin: None,
    };

    let result: Result<Conditions, _> = Some(serializable).try_into();
    assert!(result.is_err());
  }

  #[test]
  fn test_should_skip_file_match_basic() {
    let file_regex = Regex::new(r"\.js$").unwrap();
    let conditions = Conditions {
      file_match: Some(file_regex),
      code_match: None,
      enabled: true,
      origin: None,
    };

    // Should not skip when file matches
    let js_asset = create_test_asset_with_path("console.log('test')", true, "app.js");
    assert!(!conditions.should_skip(&js_asset).unwrap());

    let js_asset2 = create_test_asset_with_path("const x = 5;", true, "utils/helper.js");
    assert!(!conditions.should_skip(&js_asset2).unwrap());

    // Should skip when file doesn't match
    let ts_asset = create_test_asset_with_path("console.log('test')", true, "app.ts");
    assert!(conditions.should_skip(&ts_asset).unwrap());

    let css_asset = create_test_asset_with_path("body { color: red; }", true, "styles.css");
    assert!(conditions.should_skip(&css_asset).unwrap());
  }

  #[test]
  fn test_should_skip_file_match_complex_patterns() {
    // Test complex regex pattern: TypeScript files in src directory
    let file_regex = Regex::new(r"src/.*\.tsx?$").unwrap();
    let conditions = Conditions {
      file_match: Some(file_regex),
      code_match: None,
      enabled: true,
      origin: None,
    };

    // Should not skip matching files
    assert!(
      !conditions
        .should_skip(&create_test_asset_with_path("code", true, "src/app.ts"))
        .unwrap()
    );
    assert!(
      !conditions
        .should_skip(&create_test_asset_with_path(
          "code",
          true,
          "src/components/Button.tsx"
        ))
        .unwrap()
    );
    assert!(
      !conditions
        .should_skip(&create_test_asset_with_path(
          "code",
          true,
          "src/utils/helper.ts"
        ))
        .unwrap()
    );

    // Should skip non-matching files
    assert!(
      conditions
        .should_skip(&create_test_asset_with_path("code", true, "src/app.js"))
        .unwrap()
    ); // wrong extension
    assert!(
      conditions
        .should_skip(&create_test_asset_with_path("code", true, "lib/app.ts"))
        .unwrap()
    ); // wrong directory
    assert!(
      conditions
        .should_skip(&create_test_asset_with_path("code", true, "app.ts"))
        .unwrap()
    ); // no src/ prefix
    assert!(
      conditions
        .should_skip(&create_test_asset_with_path("code", true, "src/styles.css"))
        .unwrap()
    ); // wrong extension
  }

  #[test]
  fn test_should_skip_file_match_with_code_match() {
    let file_regex = Regex::new(r"\.js$").unwrap();
    let code_regex = RegexSet::new(["import.*react"]).unwrap();
    let conditions = Conditions {
      file_match: Some(file_regex),
      code_match: Some(code_regex),
      enabled: true,
      origin: None,
    };

    // Should not skip when both file and code match
    let matching_asset = create_test_asset_with_path("import React from 'react';", true, "app.js");
    assert!(!conditions.should_skip(&matching_asset).unwrap());

    // Should skip when file matches but code doesn't
    let file_match_only = create_test_asset_with_path("const x = 5;", true, "app.js");
    assert!(conditions.should_skip(&file_match_only).unwrap());

    // Should skip when code matches but file doesn't
    let code_match_only = create_test_asset_with_path("import React from 'react';", true, "app.ts");
    assert!(conditions.should_skip(&code_match_only).unwrap());

    // Should skip when neither matches
    let no_match = create_test_asset_with_path("const x = 5;", true, "app.ts");
    assert!(conditions.should_skip(&no_match).unwrap());
  }

  #[test]
  fn test_should_skip_file_match_with_origin() {
    let file_regex = Regex::new(r"\.js$").unwrap();
    let conditions = Conditions {
      file_match: Some(file_regex),
      code_match: None,
      enabled: true,
      origin: Some(OriginCondition::Source),
    };

    // Should not skip source JS files
    let source_js = create_test_asset_with_path("code", true, "app.js");
    assert!(!conditions.should_skip(&source_js).unwrap());

    // Should skip third-party JS files (due to origin)
    let third_party_js = create_test_asset_with_path("code", false, "app.js");
    assert!(conditions.should_skip(&third_party_js).unwrap());

    // Should skip source non-JS files (due to file_match)
    let source_ts = create_test_asset_with_path("code", true, "app.ts");
    assert!(conditions.should_skip(&source_ts).unwrap());

    // Should skip third-party non-JS files (due to both)
    let third_party_ts = create_test_asset_with_path("code", false, "app.ts");
    assert!(conditions.should_skip(&third_party_ts).unwrap());
  }

  #[test]
  fn test_should_skip_file_match_all_conditions() {
    let file_regex = Regex::new(r"src/.*\.js$").unwrap();
    let code_regex = RegexSet::new(["console\\.log"]).unwrap();
    let conditions = Conditions {
      file_match: Some(file_regex),
      code_match: Some(code_regex),
      enabled: true,
      origin: Some(OriginCondition::Source),
    };

    // Should not skip when all conditions match
    let perfect_match = create_test_asset_with_path("console.log('debug')", true, "src/debug.js");
    assert!(!conditions.should_skip(&perfect_match).unwrap());

    // Should skip when file doesn't match
    let wrong_file = create_test_asset_with_path("console.log('debug')", true, "lib/debug.js");
    assert!(conditions.should_skip(&wrong_file).unwrap());

    // Should skip when code doesn't match
    let wrong_code = create_test_asset_with_path("const x = 5;", true, "src/utils.js");
    assert!(conditions.should_skip(&wrong_code).unwrap());

    // Should skip when origin doesn't match
    let wrong_origin = create_test_asset_with_path("console.log('debug')", false, "src/debug.js");
    assert!(conditions.should_skip(&wrong_origin).unwrap());
  }

  #[test]
  fn test_should_skip_file_match_edge_cases() {
    // Test with empty string pattern (should match everything)
    let file_regex = Regex::new("").unwrap();
    let conditions = Conditions {
      file_match: Some(file_regex),
      code_match: None,
      enabled: true,
      origin: None,
    };

    let asset = create_test_asset_with_path("code", true, "any/file.ext");
    assert!(!conditions.should_skip(&asset).unwrap());

    // Test with pattern that matches directory separators
    let file_regex = Regex::new(r"/").unwrap();
    let conditions = Conditions {
      file_match: Some(file_regex),
      code_match: None,
      enabled: true,
      origin: None,
    };

    let nested_asset = create_test_asset_with_path("code", true, "src/app.js");
    assert!(!conditions.should_skip(&nested_asset).unwrap());

    let root_asset = create_test_asset_with_path("code", true, "app.js");
    assert!(conditions.should_skip(&root_asset).unwrap());
  }

  #[test]
  fn test_file_match_none_behavior() {
    // When file_match is None, file matching should be skipped entirely
    let conditions = Conditions {
      file_match: None,
      code_match: None,
      enabled: true,
      origin: None,
    };

    // Should not skip any files when file_match is None
    assert!(
      !conditions
        .should_skip(&create_test_asset_with_path("code", true, "app.js"))
        .unwrap()
    );
    assert!(
      !conditions
        .should_skip(&create_test_asset_with_path("code", true, "app.ts"))
        .unwrap()
    );
    assert!(
      !conditions
        .should_skip(&create_test_asset_with_path("code", true, "styles.css"))
        .unwrap()
    );
    assert!(
      !conditions
        .should_skip(&create_test_asset_with_path(
          "code",
          true,
          "src/nested/file.py"
        ))
        .unwrap()
    );
  }

  #[test]
  fn test_origin_condition_equality() {
    assert_eq!(OriginCondition::Source, OriginCondition::Source);
    assert_eq!(OriginCondition::ThirdParty, OriginCondition::ThirdParty);
    assert_ne!(OriginCondition::Source, OriginCondition::ThirdParty);
  }

  #[test]
  fn test_serializable_transformer_conditions_equality() {
    let cond1 = SerializableTransformerConditions {
      file_match: None,
      code_match: Some(vec!["test".to_string()]),
      enabled: Some(true),
      origin: Some(OriginCondition::Source),
    };
    let cond2 = SerializableTransformerConditions {
      file_match: None,
      code_match: Some(vec!["test".to_string()]),
      enabled: Some(true),
      origin: Some(OriginCondition::Source),
    };
    assert_eq!(cond1, cond2);
  }
}
