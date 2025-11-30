use indexmap::IndexSet;
use swc_core::common::{SyntaxContext, DUMMY_SP};
use swc_core::ecma::ast::{Expr, Ident, KeyValueProp, ObjectLit, Prop, PropName, PropOrSpread};

use crate::types::Metadata;
use crate::utils_css_map::{
  create_error_message, error_if_not_valid_object_property, get_key_value,
  has_extended_selectors_key, is_at_rule_object, is_plain_selector, object_key_is_literal_value,
  ErrorMessages,
};

fn collapse_at_rule(
  at_rule_block: &PropOrSpread,
  at_rule_type: &str,
  meta: &Metadata,
) -> Vec<(String, PropOrSpread)> {
  let PropOrSpread::Prop(prop) = at_rule_block else {
    panic!(
      "{}",
      create_error_message(ErrorMessages::NoSpreadElement.to_string())
    );
  };

  let Prop::KeyValue(key_value) = prop.as_ref() else {
    panic!(
      "{}",
      create_error_message(ErrorMessages::NoObjectMethod.to_string())
    );
  };

  let Expr::Object(object) = key_value.value.as_ref() else {
    panic!(
      "{}",
      create_error_message(ErrorMessages::AtRuleValueType.to_string())
    );
  };

  let mut collapsed = Vec::new();

  for entry in &object.props {
    error_if_not_valid_object_property(entry, meta);

    let PropOrSpread::Prop(entry_prop) = entry else {
      continue;
    };

    let Prop::KeyValue(entry_key_value) = entry_prop.as_ref() else {
      continue;
    };

    if !object_key_is_literal_value(&entry_key_value.key) {
      panic!(
        "{}",
        create_error_message(ErrorMessages::StaticPropertyKey.to_string())
      );
    }

    let at_rule_suffix = get_key_value(&entry_key_value.key);
    let at_rule_name = format!("{} {}", at_rule_type, at_rule_suffix);
    let ident = Ident::new(
      at_rule_name.clone().into(),
      DUMMY_SP,
      SyntaxContext::empty(),
    );
    let new_prop = Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(ident.into()),
      value: entry_key_value.value.clone(),
    });

    collapsed.push((at_rule_name, PropOrSpread::Prop(Box::new(new_prop))));
  }

  collapsed
}

fn get_extended_selectors(variant_styles: &ObjectLit, meta: &Metadata) -> Vec<PropOrSpread> {
  let extended: Vec<&PropOrSpread> = variant_styles
    .props
    .iter()
    .filter(|prop| has_extended_selectors_key(prop))
    .collect();

  if extended.is_empty() {
    return Vec::new();
  }

  if extended.len() > 1 {
    panic!(
      "{}",
      create_error_message(ErrorMessages::DuplicateSelectorsBlock.to_string())
    );
  }

  error_if_not_valid_object_property(extended[0], meta);

  let PropOrSpread::Prop(prop) = extended[0] else {
    panic!(
      "{}",
      create_error_message(ErrorMessages::NoSpreadElement.to_string())
    );
  };

  let Prop::KeyValue(key_value) = prop.as_ref() else {
    panic!(
      "{}",
      create_error_message(ErrorMessages::NoObjectMethod.to_string())
    );
  };

  let Expr::Object(object) = key_value.value.as_ref() else {
    panic!(
      "{}",
      create_error_message(ErrorMessages::SelectorsBlockValueType.to_string())
    );
  };

  object.props.clone()
}

/// Merges extended selector declarations (selectors/at-rules) into the root object so
/// downstream CSS builders can treat them like standard properties.
pub fn merge_extended_selectors_into_properties(
  variant_styles: &ObjectLit,
  meta: &Metadata,
) -> ObjectLit {
  let extended = get_extended_selectors(variant_styles, meta);
  let mut merged_properties: Vec<PropOrSpread> = Vec::new();
  let mut added_selectors: IndexSet<String> = IndexSet::new();

  for property in variant_styles.props.iter().chain(extended.iter()) {
    error_if_not_valid_object_property(property, meta);

    if has_extended_selectors_key(property) {
      continue;
    }

    let PropOrSpread::Prop(prop) = property else {
      continue;
    };

    let Prop::KeyValue(key_value) = prop.as_ref() else {
      continue;
    };

    if !object_key_is_literal_value(&key_value.key) {
      panic!(
        "{}",
        create_error_message(ErrorMessages::StaticPropertyKey.to_string())
      );
    }

    let key = get_key_value(&key_value.key);

    if is_plain_selector(&key) {
      panic!(
        "{}",
        create_error_message(ErrorMessages::UseSelectorsWithAmpersand.to_string())
      );
    }

    if is_at_rule_object(&key_value.key) {
      for (at_rule_name, collapsed_prop) in collapse_at_rule(property, &key, meta) {
        if !added_selectors.insert(at_rule_name.clone()) {
          panic!(
            "{}",
            create_error_message(ErrorMessages::DuplicateAtRule.to_string())
          );
        }

        merged_properties.push(collapsed_prop);
      }

      continue;
    }

    let is_selector = matches!(key_value.value.as_ref(), Expr::Object(_));

    if is_selector {
      if !added_selectors.insert(key.clone()) {
        panic!(
          "{}",
          create_error_message(ErrorMessages::DuplicateSelector.to_string())
        );
      }
    }

    merged_properties.push(property.clone());
  }

  ObjectLit {
    span: variant_styles.span,
    props: merged_properties,
  }
}

#[cfg(test)]
mod tests {
  use super::merge_extended_selectors_into_properties;
  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};
  use std::cell::RefCell;
  use std::panic::AssertUnwindSafe;
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{SourceMap, SyntaxContext, DUMMY_SP};
  use swc_core::ecma::ast::{
    Expr, Ident, KeyValueProp, Lit, Number, ObjectLit, Prop, PropName, PropOrSpread, Str,
  };

  fn create_metadata() -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::with_options(
      cm.clone(),
      Vec::new(),
      crate::types::TransformFileOptions {
        filename: Some("test.tsx".into()),
        cwd: Some(std::env::current_dir().expect("cwd")),
        root: Some(std::env::current_dir().expect("cwd")),
        ..Default::default()
      },
    );
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));

    Metadata::new(state)
  }

  fn ident(name: &str) -> PropName {
    PropName::Ident(Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty()).into())
  }

  fn string_lit(value: &str) -> Expr {
    Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: value.into(),
      raw: None,
    }))
  }

  fn number_lit(value: f64) -> Expr {
    Expr::Lit(Lit::Num(Number {
      span: DUMMY_SP,
      value,
      raw: None,
    }))
  }

  #[test]
  fn merges_extended_selectors_into_properties() {
    let meta = create_metadata();
    let variant_styles = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("color"),
          value: Box::new(string_lit("blue")),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("@media"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
              key: ident("screen and (min-width: 500px)"),
              value: Box::new(string_lit("display: block")),
            })))],
          })),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("selectors"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
              key: ident("div"),
              value: Box::new(Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                  key: ident("margin"),
                  value: Box::new(number_lit(0.0)),
                })))],
              })),
            })))],
          })),
        }))),
      ],
    };

    let merged = merge_extended_selectors_into_properties(&variant_styles, &meta);
    let keys: Vec<String> = merged
      .props
      .iter()
      .map(|prop| match prop {
        PropOrSpread::Prop(prop) => match prop.as_ref() {
          Prop::KeyValue(kv) => get_key(kv),
          _ => panic!("expected key value"),
        },
        _ => panic!("expected prop"),
      })
      .collect();

    assert_eq!(
      keys,
      vec![
        "color".to_string(),
        "@media screen and (min-width: 500px)".to_string(),
        "div".to_string()
      ]
    );
  }

  fn get_key(prop: &KeyValueProp) -> String {
    match &prop.key {
      PropName::Ident(ident) => ident.sym.to_string(),
      PropName::Str(str) => str.value.to_string(),
      _ => panic!("unexpected key type"),
    }
  }

  #[test]
  fn detects_duplicate_selectors() {
    let meta = create_metadata();
    let variant_styles = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("div"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![],
          })),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("div"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![],
          })),
        }))),
      ],
    };

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
      merge_extended_selectors_into_properties(&variant_styles, &meta);
    }));

    let panic_message = match result {
      Ok(_) => panic!("expected panic"),
      Err(err) => {
        if let Some(msg) = err.downcast_ref::<String>() {
          msg.clone()
        } else if let Some(msg) = err.downcast_ref::<&'static str>() {
          (*msg).to_string()
        } else {
          String::new()
        }
      }
    };

    assert!(panic_message.contains("Cannot declare a selector more than once"));
  }

  #[test]
  fn detects_duplicate_selectors_block() {
    let meta = create_metadata();
    let variant_styles = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("selectors"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![],
          })),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("selectors"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![],
          })),
        }))),
      ],
    };

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
      merge_extended_selectors_into_properties(&variant_styles, &meta);
    }));

    let panic_message = match result {
      Ok(_) => panic!("expected panic"),
      Err(err) => {
        if let Some(msg) = err.downcast_ref::<String>() {
          msg.clone()
        } else if let Some(msg) = err.downcast_ref::<&'static str>() {
          (*msg).to_string()
        } else {
          String::new()
        }
      }
    };

    assert!(panic_message.contains("Duplicate `selectors` key"));
  }
}
