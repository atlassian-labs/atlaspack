use indexmap::IndexSet;
use swc_core::common::{DUMMY_SP, Spanned, SyntaxContext};
use swc_core::ecma::ast::{Expr, Ident, KeyValueProp, ObjectLit, Prop, PropName, PropOrSpread};

use crate::types::Metadata;
use crate::utils_css_map::{
  ErrorMessages, create_css_map_diagnostic, create_css_map_diagnostic_with_hints,
  error_if_not_valid_object_property, get_key_value, has_extended_selectors_key, is_at_rule_object,
  is_plain_selector, object_key_is_literal_value, report_css_map_error_with_hints,
};

fn collapse_at_rule(
  at_rule_block: &PropOrSpread,
  at_rule_type: &str,
  meta: &Metadata,
) -> Vec<(String, PropOrSpread)> {
  let PropOrSpread::Prop(prop) = at_rule_block else {
    meta.add_diagnostic(create_css_map_diagnostic_with_hints(
      ErrorMessages::NoSpreadElement,
    ));
    return Vec::new();
  };

  let Prop::KeyValue(key_value) = prop.as_ref() else {
    meta.add_diagnostic(create_css_map_diagnostic_with_hints(
      ErrorMessages::NoObjectMethod,
    ));
    return Vec::new();
  };

  let Expr::Object(object) = key_value.value.as_ref() else {
    meta.add_diagnostic(create_css_map_diagnostic(ErrorMessages::AtRuleValueType));
    return Vec::new();
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
      report_css_map_error_with_hints(
        meta,
        entry_key_value.key.span(),
        ErrorMessages::StaticAtRuleKey,
      );
      continue;
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
    meta.add_diagnostic(create_css_map_diagnostic(
      ErrorMessages::DuplicateSelectorsBlock,
    ));
    return Vec::new();
  }

  error_if_not_valid_object_property(extended[0], meta);

  let PropOrSpread::Prop(prop) = extended[0] else {
    meta.add_diagnostic(create_css_map_diagnostic_with_hints(
      ErrorMessages::NoSpreadElement,
    ));
    return Vec::new();
  };

  let Prop::KeyValue(key_value) = prop.as_ref() else {
    meta.add_diagnostic(create_css_map_diagnostic_with_hints(
      ErrorMessages::NoObjectMethod,
    ));
    return Vec::new();
  };

  let Expr::Object(object) = key_value.value.as_ref() else {
    meta.add_diagnostic(create_css_map_diagnostic(
      ErrorMessages::SelectorsBlockValueType,
    ));
    return Vec::new();
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
      report_css_map_error_with_hints(meta, key_value.key.span(), ErrorMessages::StaticSelectorKey);
      continue;
    }

    let key = get_key_value(&key_value.key);

    if is_plain_selector(&key) {
      report_css_map_error_with_hints(
        meta,
        key_value.key.span(),
        ErrorMessages::UseSelectorsWithAmpersand,
      );
      continue;
    }

    if is_at_rule_object(&key_value.key) {
      for (at_rule_name, collapsed_prop) in collapse_at_rule(property, &key, meta) {
        if !added_selectors.insert(at_rule_name.clone()) {
          report_css_map_error_with_hints(
            meta,
            key_value.key.span(),
            ErrorMessages::DuplicateAtRule,
          );
          continue;
        }

        merged_properties.push(collapsed_prop);
      }

      continue;
    }

    let is_selector = matches!(key_value.value.as_ref(), Expr::Object(_));

    if is_selector {
      if !added_selectors.insert(key.clone()) {
        report_css_map_error_with_hints(
          meta,
          key_value.key.span(),
          ErrorMessages::DuplicateSelector,
        );
        continue;
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
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{DUMMY_SP, SourceMap, SyntaxContext};
  use swc_core::ecma::ast::{
    Expr, Ident, KeyValueProp, Lit, Number, ObjectLit, Prop, PropName, PropOrSpread, Str,
  };

  fn create_metadata() -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::transform_compiled_with_options(
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

    merge_extended_selectors_into_properties(&variant_styles, &meta);

    let diagnostics = meta.state().diagnostics.clone();
    assert_eq!(diagnostics.len(), 1);
    assert!(
      diagnostics[0]
        .message
        .contains("Cannot declare a selector more than once")
    );
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

    merge_extended_selectors_into_properties(&variant_styles, &meta);

    let diagnostics = meta.state().diagnostics.clone();
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("Duplicate `selectors` key"));
  }

  fn string_key(name: &str) -> PropName {
    PropName::Str(Str {
      span: DUMMY_SP,
      value: name.into(),
      raw: None,
    })
  }

  #[test]
  fn merges_div_selector_correctly() {
    let meta = create_metadata();
    let variant_styles = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("color"),
          value: Box::new(string_lit("blue")),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: string_key("div"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
              key: ident("margin"),
              value: Box::new(number_lit(0.0)),
            })))],
          })),
        }))),
      ],
    };

    let merged = merge_extended_selectors_into_properties(&variant_styles, &meta);

    // Should have both the color property and the div selector
    assert_eq!(merged.props.len(), 2);

    // Verify the keys
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

    assert!(keys.contains(&"color".to_string()));
    assert!(keys.contains(&"div".to_string()));
  }

  #[test]
  fn merges_span_selector_correctly() {
    let meta = create_metadata();
    let variant_styles = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("display"),
          value: Box::new(string_lit("flex")),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: string_key("span"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
              key: ident("fontWeight"),
              value: Box::new(string_lit("bold")),
            })))],
          })),
        }))),
      ],
    };

    let merged = merge_extended_selectors_into_properties(&variant_styles, &meta);
    assert_eq!(merged.props.len(), 2);
  }

  #[test]
  fn merges_multiple_element_selectors() {
    let meta = create_metadata();
    let variant_styles = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("padding"),
          value: Box::new(string_lit("8px")),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: string_key("div"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
              key: ident("margin"),
              value: Box::new(string_lit("0")),
            })))],
          })),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: string_key("button"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
              key: ident("cursor"),
              value: Box::new(string_lit("pointer")),
            })))],
          })),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: string_key("input"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
              key: ident("border"),
              value: Box::new(string_lit("1px solid")),
            })))],
          })),
        }))),
      ],
    };

    let merged = merge_extended_selectors_into_properties(&variant_styles, &meta);

    // Should have padding + div + button + input = 4 properties
    assert_eq!(merged.props.len(), 4);
  }

  #[test]
  fn merges_ampersand_hover_selector() {
    let meta = create_metadata();
    let variant_styles = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("backgroundColor"),
          value: Box::new(string_lit("white")),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: string_key("&:hover"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
              key: ident("backgroundColor"),
              value: Box::new(string_lit("lightgray")),
            })))],
          })),
        }))),
      ],
    };

    let merged = merge_extended_selectors_into_properties(&variant_styles, &meta);
    assert_eq!(merged.props.len(), 2);

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

    assert!(keys.contains(&"backgroundColor".to_string()));
    assert!(keys.contains(&"&:hover".to_string()));
  }

  #[test]
  fn merges_extended_selectors_with_div() {
    let meta = create_metadata();
    let variant_styles = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("color"),
          value: Box::new(string_lit("black")),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("selectors"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![
              PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: string_key("& div"),
                value: Box::new(Expr::Object(ObjectLit {
                  span: DUMMY_SP,
                  props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: ident("marginTop"),
                    value: Box::new(string_lit("8px")),
                  })))],
                })),
              }))),
              PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: string_key("& > div"),
                value: Box::new(Expr::Object(ObjectLit {
                  span: DUMMY_SP,
                  props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: ident("padding"),
                    value: Box::new(string_lit("4px")),
                  })))],
                })),
              }))),
            ],
          })),
        }))),
      ],
    };

    let merged = merge_extended_selectors_into_properties(&variant_styles, &meta);

    // Should have: color + "& div" + "& > div" = 3 properties
    assert_eq!(merged.props.len(), 3);

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

    assert!(keys.contains(&"color".to_string()));
    assert!(keys.contains(&"& div".to_string()));
    assert!(keys.contains(&"& > div".to_string()));
  }

  #[test]
  fn merges_complex_nested_selectors() {
    let meta = create_metadata();
    let variant_styles = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("display"),
          value: Box::new(string_lit("grid")),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: string_key("& div:first-child"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
              key: ident("gridColumn"),
              value: Box::new(string_lit("1 / -1")),
            })))],
          })),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: string_key("& div:last-child"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
              key: ident("marginBottom"),
              value: Box::new(number_lit(0.0)),
            })))],
          })),
        }))),
      ],
    };

    let merged = merge_extended_selectors_into_properties(&variant_styles, &meta);
    assert_eq!(merged.props.len(), 3);
  }

  #[test]
  fn handles_empty_selectors_block() {
    let meta = create_metadata();
    let variant_styles = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("color"),
          value: Box::new(string_lit("red")),
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

    let merged = merge_extended_selectors_into_properties(&variant_styles, &meta);

    // Should only have the color property since selectors is empty
    assert_eq!(merged.props.len(), 1);

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

    assert_eq!(keys, vec!["color".to_string()]);
  }

  #[test]
  fn preserves_property_order() {
    let meta = create_metadata();
    let variant_styles = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("zIndex"),
          value: Box::new(number_lit(1.0)),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("position"),
          value: Box::new(string_lit("relative")),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: string_key("& span"),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
              key: ident("color"),
              value: Box::new(string_lit("inherit")),
            })))],
          })),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: ident("opacity"),
          value: Box::new(number_lit(1.0)),
        }))),
      ],
    };

    let merged = merge_extended_selectors_into_properties(&variant_styles, &meta);
    assert_eq!(merged.props.len(), 4);

    // Verify order is preserved
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

    assert_eq!(keys[0], "zIndex");
    assert_eq!(keys[1], "position");
    assert_eq!(keys[2], "& span");
    assert_eq!(keys[3], "opacity");
  }

  #[test]
  fn diagnostics_include_span_information() {
    use swc_core::common::BytePos;

    let meta = create_metadata();

    // Create a property with a non-dummy span to test that span information is preserved
    let test_span = swc_core::common::Span::new(BytePos(10), BytePos(20));

    let variant_styles = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: PropName::Ident(Ident::new("div".into(), test_span, SyntaxContext::empty()).into()),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![],
          })),
        }))),
        // Duplicate selector to trigger diagnostic
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: PropName::Ident(Ident::new("div".into(), test_span, SyntaxContext::empty()).into()),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![],
          })),
        }))),
      ],
    };

    merge_extended_selectors_into_properties(&variant_styles, &meta);

    let diagnostics = meta.state().diagnostics.clone();
    assert_eq!(diagnostics.len(), 1);

    // Verify the diagnostic has a non-empty span
    let diagnostic = &diagnostics[0];
    assert!(
      diagnostic
        .message
        .contains("Cannot declare a selector more than once")
    );

    // Verify span information is present and non-dummy
    assert!(diagnostic.span.is_some());
    let span = diagnostic.span.unwrap();
    assert_ne!(span.lo, BytePos(0));
    assert_ne!(span.hi, BytePos(0));
    assert_eq!(span.lo, BytePos(10));
    assert_eq!(span.hi, BytePos(20));
  }

  #[test]
  fn diagnostics_with_sourcemap_snippet() {
    use swc_core::common::{BytePos, FileName, SourceMapper};

    // Create a SourceMap with actual source code to simulate real file processing
    let cm: Lrc<SourceMap> = Default::default();
    let source_code = "cssMap({ container: { div: {}, div: {} } })";

    let source_file = cm.new_source_file(
      FileName::Custom("test.tsx".into()).into(),
      source_code.to_string(),
    );

    // Create an AST with spans that reference actual source positions
    // Simulate the "div" key appearing at byte positions 22-25 and 31-34 in the source
    let first_div_span = swc_core::common::Span::new(
      source_file.start_pos + BytePos(22),
      source_file.start_pos + BytePos(25),
    );
    let second_div_span = swc_core::common::Span::new(
      source_file.start_pos + BytePos(31),
      source_file.start_pos + BytePos(34),
    );

    // Create metadata with the real source map
    let file = TransformFile::transform_compiled_with_options(
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
    let meta = Metadata::new(state);

    // Create a variant_styles object with duplicate div selectors using real spans
    let variant_styles = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: PropName::Ident(
            Ident::new("div".into(), first_div_span, SyntaxContext::empty()).into(),
          ),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![],
          })),
        }))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key: PropName::Ident(
            Ident::new("div".into(), second_div_span, SyntaxContext::empty()).into(),
          ),
          value: Box::new(Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![],
          })),
        }))),
      ],
    };

    // Process the object which has duplicate "div" keys
    merge_extended_selectors_into_properties(&variant_styles, &meta);

    let diagnostics = meta.state().diagnostics.clone();
    assert_eq!(
      diagnostics.len(),
      1,
      "Expected one diagnostic for duplicate selector"
    );

    let diagnostic = &diagnostics[0];
    assert!(
      diagnostic
        .message
        .contains("Cannot declare a selector more than once")
    );

    // Verify the diagnostic has span information that can be used to generate a code frame
    assert!(
      diagnostic.span.is_some(),
      "Diagnostic should have span information"
    );
    let span = diagnostic.span.unwrap();

    // The span should point to actual source code (not at position 0)
    assert_ne!(
      span.lo, source_file.start_pos,
      "Span should not be at file start"
    );

    // Verify we can retrieve the source code for this span using the source map
    let span_snippet = cm.span_to_snippet(span);
    assert!(
      span_snippet.is_ok(),
      "Should be able to get source snippet from span: {:?}",
      span_snippet.err()
    );

    let snippet = span_snippet.unwrap();
    assert_eq!(snippet, "div", "Snippet should be exactly 'div'");
  }
}
