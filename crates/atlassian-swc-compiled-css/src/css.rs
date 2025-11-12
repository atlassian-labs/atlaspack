use crate::{CompiledCssInJsCollector, utils::rule_hash};

const CSS_PREFIX: &str = "_";

impl CompiledCssInJsCollector {
  pub fn hash_ix(&self, key: &str) -> String {
    rule_hash(key)
  }

  pub fn hash_class_name(
    &self,
    property_name: &str,
    property_value: &str,
    selectors_str: &str,
    at_rule: Option<&str>,
    important: bool,
  ) -> String {
    let mut result: String = CSS_PREFIX.to_string();

    // opts.atRule + selectors + node.prop
    let name_hash = rule_hash(&format!(
      "{}{}{}",
      at_rule.unwrap_or("undefined"),
      selectors_str,
      property_name
    ));
    result.push_str(&name_hash[..4]);

    let important_str = if important { "true" } else { "" };
    // property_value + important_str
    let value_hash = rule_hash(&format!(
      "{}{}",
      if property_value.is_empty() {
        // We turn empty property values into empty string types in the hash
        "\"\""
      } else {
        property_value
      },
      important_str
    ));
    result.push_str(&value_hash[..4]);

    result
  }

  /// Normalise CSS property value to stable values
  /// This is important so that functionally equivalent CSS properties
  /// are hashed to the same value
  pub fn normalise_css_property_value(&self, css: &str) -> String {
    if css.starts_with("#") {
      css.to_lowercase()
    } else {
      css.to_string()
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::tests::utils::{assert_contains, transform_code};

  #[test]
  fn test_basic_css() {
    let code = r#"
    import { css } from '@compiled/react';

    const styles = css({
      color: 'blue',
    });

    <div css={styles} />;
    "#;

    let (actual_code, _result) = transform_code(code, "test.jsx", None);

    assert_contains(&actual_code, "._syaz13q2{color:blue}");
  }

  #[test]
  fn test_normalise_css_property_value() {
    let css = "#FFF";
    let v = CompiledCssInJsCollector::default();
    let actual_code = v.normalise_css_property_value(css);
    assert_eq!(actual_code, "#fff");
  }
}
