use super::types::{parse_value_to_components, LonghandDeclaration, ValueNode, ValuesRoot};
use super::utils::{get_width, is_width};

const FLEX_BASIS_DEFAULT: &str = "0%";

/// Expand the `flex` shorthand following the specification semantics.
pub fn flex(value: &ValuesRoot) -> Vec<LonghandDeclaration> {
  if value.nodes.is_empty() {
    return vec![LonghandDeclaration::keep_original()];
  }

  let left = &value.nodes[0];
  let middle = value.nodes.get(1);
  let right = value.nodes.get(2);

  match value.nodes.len() {
    1 => expand_single(left),
    2 => expand_double(left, middle.unwrap()),
    3 => match expand_triple(left, middle.unwrap(), right.unwrap()) {
      Some(result) => result,
      None => Vec::new(),
    },
    _ => Vec::new(),
  }
}

fn expand_single(node: &ValueNode) -> Vec<LonghandDeclaration> {
  if let Some(word) = node.as_word() {
    match word {
      "auto" => {
        return vec![
          LonghandDeclaration::replace("flex-grow", parse_value_to_components("1")),
          LonghandDeclaration::replace("flex-shrink", parse_value_to_components("1")),
          LonghandDeclaration::replace("flex-basis", parse_value_to_components("auto")),
        ];
      }
      "none" => {
        return vec![
          LonghandDeclaration::replace("flex-grow", parse_value_to_components("0")),
          LonghandDeclaration::replace("flex-shrink", parse_value_to_components("0")),
          LonghandDeclaration::replace("flex-basis", parse_value_to_components("auto")),
        ];
      }
      "initial" => {
        return vec![
          LonghandDeclaration::replace("flex-grow", parse_value_to_components("0")),
          LonghandDeclaration::replace("flex-shrink", parse_value_to_components("1")),
          LonghandDeclaration::replace("flex-basis", parse_value_to_components("auto")),
        ];
      }
      "revert" | "revert-layer" | "unset" | "inherit" => {
        return vec![LonghandDeclaration::keep_original()];
      }
      _ => {}
    }
  }

  if is_flex_number(node) {
    if let Some(number) = node.numeric_string() {
      return vec![
        LonghandDeclaration::replace("flex-grow", parse_value_to_components(&number)),
        LonghandDeclaration::replace("flex-shrink", parse_value_to_components("1")),
        LonghandDeclaration::replace("flex-basis", parse_value_to_components(FLEX_BASIS_DEFAULT)),
      ];
    }
  }

  if is_flex_basis(node) {
    if let Some(width) = flex_basis_value(node) {
      return vec![
        LonghandDeclaration::replace("flex-grow", parse_value_to_components("1")),
        LonghandDeclaration::replace("flex-shrink", parse_value_to_components("1")),
        LonghandDeclaration::replace("flex-basis", parse_value_to_components(&width)),
      ];
    }
  }

  Vec::new()
}

fn expand_double(left: &ValueNode, right: &ValueNode) -> Vec<LonghandDeclaration> {
  if is_flex_number(left) {
    if is_flex_number(right) {
      if let (Some(grow), Some(shrink)) = (left.numeric_string(), right.numeric_string()) {
        return vec![
          LonghandDeclaration::replace("flex-grow", parse_value_to_components(&grow)),
          LonghandDeclaration::replace("flex-shrink", parse_value_to_components(&shrink)),
          LonghandDeclaration::replace("flex-basis", parse_value_to_components(FLEX_BASIS_DEFAULT)),
        ];
      }
    } else if is_flex_basis(right) {
      if let Some(grow) = left.numeric_string() {
        if let Some(width) = flex_basis_value(right) {
          return vec![
            LonghandDeclaration::replace("flex-grow", parse_value_to_components(&grow)),
            LonghandDeclaration::replace("flex-shrink", parse_value_to_components("1")),
            LonghandDeclaration::replace("flex-basis", parse_value_to_components(&width)),
          ];
        }
      }
    }
  }

  Vec::new()
}

fn expand_triple(
  left: &ValueNode,
  middle: &ValueNode,
  right: &ValueNode,
) -> Option<Vec<LonghandDeclaration>> {
  if is_flex_number(left) && is_flex_number(middle) && is_flex_basis(right) {
    let grow = left.numeric_string()?;
    let shrink = middle.numeric_string()?;
    let basis = flex_basis_value(right)?;

    return Some(vec![
      LonghandDeclaration::replace("flex-grow", parse_value_to_components(&grow)),
      LonghandDeclaration::replace("flex-shrink", parse_value_to_components(&shrink)),
      LonghandDeclaration::replace("flex-basis", parse_value_to_components(&basis)),
    ]);
  }

  None
}

fn is_flex_number(node: &ValueNode) -> bool {
  node.is_numeric_without_unit()
}

fn is_flex_basis(node: &ValueNode) -> bool {
  if let Some(word) = node.as_word() {
    if word == "content" {
      return true;
    }
  }

  if node.is_unitless_zero() {
    return true;
  }

  is_width(node)
}

fn flex_basis_value(node: &ValueNode) -> Option<String> {
  if node.is_unitless_zero() {
    return Some(FLEX_BASIS_DEFAULT.to_string());
  }

  if let Some(width) = get_width(node) {
    return Some(width);
  }

  if let Some(word) = node.as_word() {
    return Some(word.to_string());
  }

  node.to_css_string()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::plugins::expand_shorthands::types::{
    parse_value_to_components, serialize_component_values,
  };

  fn stringify(declarations: &[LonghandDeclaration]) -> Vec<(String, String)> {
    declarations
      .iter()
      .map(|entry| match entry {
        LonghandDeclaration::Replace { prop, value } => {
          (prop.clone(), serialize_component_values(value).unwrap())
        }
        LonghandDeclaration::KeepOriginal => ("keep".into(), String::new()),
      })
      .collect()
  }

  #[test]
  fn expands_auto_keyword() {
    let root = ValuesRoot::from_components(&parse_value_to_components("auto"));
    let result = flex(&root);
    assert_eq!(
      stringify(&result),
      vec![
        ("flex-grow".into(), "1".into()),
        ("flex-shrink".into(), "1".into()),
        ("flex-basis".into(), "auto".into()),
      ]
    );
  }

  #[test]
  fn expands_numeric_single_value() {
    let root = ValuesRoot::from_components(&parse_value_to_components("2"));
    let result = flex(&root);
    assert_eq!(
      stringify(&result),
      vec![
        ("flex-grow".into(), "2".into()),
        ("flex-shrink".into(), "1".into()),
        ("flex-basis".into(), FLEX_BASIS_DEFAULT.into()),
      ]
    );
  }

  #[test]
  fn expands_three_values() {
    let root = ValuesRoot::from_components(&parse_value_to_components("1 0 20px"));
    let result = flex(&root);
    assert_eq!(
      stringify(&result),
      vec![
        ("flex-grow".into(), "1".into()),
        ("flex-shrink".into(), "0".into()),
        ("flex-basis".into(), "20px".into()),
      ]
    );
  }

  #[test]
  fn returns_empty_for_invalid() {
    let root = ValuesRoot::from_components(&parse_value_to_components("foo bar"));
    let result = flex(&root);
    assert!(result.is_empty());
  }
}
