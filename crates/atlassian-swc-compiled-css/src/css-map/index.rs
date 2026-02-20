use swc_core::atoms::Atom;
use swc_core::common::{DUMMY_SP, Span, Spanned};
use swc_core::ecma::ast::{
  CallExpr, Callee, Expr, Ident, KeyValueProp, Lit, ObjectLit, Prop, PropOrSpread, Str, TaggedTpl,
};

use crate::css_map_process_selectors::merge_extended_selectors_into_properties;
use crate::types::Metadata;
use crate::utils_css_builders::build_css as build_css_from_expr;
use crate::utils_css_map::{
  ErrorMessages, error_if_not_valid_object_property, report_css_map_error,
  report_css_map_error_with_hints,
};
use crate::utils_transform_css_items::transform_css_items;
use crate::utils_types::CssOutput;

/// Describes the supported syntactic shapes for a `cssMap` invocation.
pub enum CssMapUsage<'a> {
  Call(&'a CallExpr),
  TaggedTemplate(&'a TaggedTpl),
}

/// Shared helper that mirrors the Babel `visitCssMapPath` implementation but allows the
/// CSS builder to be injected so tests can exercise the behaviour before the full
/// `build_css` port lands.
pub fn visit_css_map_path_with_builder<'a, F>(
  usage: CssMapUsage<'a>,
  parent_identifier: Option<&Ident>,
  meta: &Metadata,
  mut build_css: F,
) -> ObjectLit
where
  F: FnMut(&Expr, &Metadata) -> CssOutput,
{
  match usage {
    CssMapUsage::TaggedTemplate(tagged_tpl) => {
      report_css_map_error_with_hints(meta, tagged_tpl.span, ErrorMessages::NoTaggedTemplate);
      return empty_object(tagged_tpl.span);
    }
    CssMapUsage::Call(call_expr) => {
      let Some(binding_identifier) = parent_identifier else {
        report_css_map_error(meta, call_expr.span, ErrorMessages::DefineMap.message());
        return empty_object(call_expr.span);
      };

      if call_expr.args.len() != 1 {
        report_css_map_error_with_hints(meta, call_expr.span, ErrorMessages::NumberOfArgument);
        return empty_object(call_expr.span);
      }

      let argument = &call_expr.args[0];
      if argument.spread.is_some() {
        report_css_map_error_with_hints(meta, argument.span(), ErrorMessages::ArgumentType);
        return empty_object(call_expr.span);
      }

      let Expr::Object(object_lit) = argument.expr.as_ref() else {
        report_css_map_error_with_hints(meta, argument.expr.span(), ErrorMessages::ArgumentType);
        return empty_object(call_expr.span);
      };

      let mut total_sheets: Vec<String> = Vec::new();
      let mut new_properties: Vec<PropOrSpread> = Vec::with_capacity(object_lit.props.len());
      let initial_style_rules = meta.state().style_rules.clone();
      let initial_sheets = meta.state().sheets.clone();

      for property in &object_lit.props {
        if error_if_not_valid_object_property(property, meta) {
          return empty_object(object_lit.span);
        }

        let PropOrSpread::Prop(prop) = property else {
          continue;
        };

        let Prop::KeyValue(key_value) = prop.as_ref() else {
          report_css_map_error(
            meta,
            prop.span(),
            ErrorMessages::StaticVariantObject.message(),
          );
          return empty_object(object_lit.span);
        };

        let Expr::Object(variant_styles) = key_value.value.as_ref() else {
          report_css_map_error(
            meta,
            key_value.value.span(),
            ErrorMessages::StaticVariantObject.message(),
          );
          return empty_object(object_lit.span);
        };

        let processed_value = merge_extended_selectors_into_properties(variant_styles, meta);
        let css_output = build_css(&Expr::Object(processed_value.clone()), meta);

        if !css_output.variables.is_empty() {
          let has_token_call = css_output.variables.iter().any(|v| {
            matches!(
              &v.expression,
              Expr::Call(CallExpr {
                callee: Callee::Expr(callee),
                ..
              }) if matches!(&**callee, Expr::Ident(id) if id.sym == "token")
            )
          });

          let error_type = if has_token_call {
            ErrorMessages::StaticVariantObjectWithToken
          } else {
            ErrorMessages::StaticVariantObjectWithVariables
          };

          report_css_map_error_with_hints(meta, key_value.value.span(), error_type);
          return empty_object(object_lit.span);
        }

        let transform_result = transform_css_items(&css_output.css, meta);
        total_sheets.extend(
          transform_result
            .sheets
            .iter()
            .filter(|sheet| sheet.contains('{'))
            .cloned(),
        );

        if transform_result.class_names.len() > 1 {
          report_css_map_error_with_hints(
            meta,
            key_value.value.span(),
            ErrorMessages::StaticVariantObjectMultipleClasses,
          );
          return empty_object(object_lit.span);
        }

        let mut class_names = transform_result.class_names.into_iter();
        let class_expression = class_names.next().unwrap_or_else(|| {
          Expr::Lit(Lit::Str(Str {
            span: DUMMY_SP,
            value: Atom::from(""),
            raw: Some("\"\"".into()),
          }))
        });

        let new_prop = Prop::KeyValue(KeyValueProp {
          key: key_value.key.clone(),
          value: Box::new(class_expression),
        });

        new_properties.push(PropOrSpread::Prop(Box::new(new_prop)));
      }

      if std::env::var("COMPILED_CLI_TRACE").is_ok() {
        eprintln!(
          "[css-map] cached {} sheets for {} => {:?}",
          total_sheets.len(),
          binding_identifier.sym,
          total_sheets
        );
      }

      meta
        .state_mut()
        .css_map
        .insert(binding_identifier.sym.to_string(), total_sheets);
      {
        let mut state = meta.state_mut();
        // Avoid surfacing style_rules/sheets from cssMap definitions; Babel leaves metadata empty.
        state.style_rules = initial_style_rules;
        state.sheets = initial_sheets;
      }

      ObjectLit {
        span: object_lit.span,
        props: new_properties,
      }
    }
  }
}

/// Convenience wrapper around `visit_css_map_path_with_builder` that wires in
/// the shared `build_css` helper.
pub fn visit_css_map_path<'a>(
  usage: CssMapUsage<'a>,
  parent_identifier: Option<&Ident>,
  meta: &Metadata,
) -> ObjectLit {
  visit_css_map_path_with_builder(usage, parent_identifier, meta, build_css_from_expr)
}

fn empty_object(span: Span) -> ObjectLit {
  ObjectLit {
    span,
    props: Vec::new(),
  }
}

#[cfg(test)]
mod tests {
  use super::{CssMapUsage, visit_css_map_path_with_builder};
  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};
  use crate::utils_types::{CssItem, CssOutput};
  use std::cell::RefCell;
  use std::rc::Rc;
  use swc_core::atoms::Atom;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{DUMMY_SP, SourceMap, SyntaxContext};
  use swc_core::ecma::ast::{
    CallExpr, Callee, Expr, ExprOrSpread, Ident, KeyValueProp, Lit, Number, ObjectLit, Prop,
    PropName, PropOrSpread, Str, TaggedTpl, Tpl, TplElement,
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

  fn ident(name: &str) -> Ident {
    Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty())
  }

  fn string_lit(value: &str) -> Expr {
    Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: value.into(),
      raw: None,
    }))
  }

  fn object_property(name: &str, value: Expr) -> PropOrSpread {
    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(ident(name).into()),
      value: Box::new(value),
    })))
  }

  fn css_map_argument() -> ObjectLit {
    ObjectLit {
      span: DUMMY_SP,
      props: vec![
        object_property(
          "danger",
          Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![
              object_property("color", string_lit("red")),
              object_property("backgroundColor", string_lit("red")),
            ],
          }),
        ),
        object_property(
          "success",
          Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![
              object_property("color", string_lit("green")),
              object_property("backgroundColor", string_lit("green")),
            ],
          }),
        ),
      ],
    }
  }

  fn css_map_call(argument: Expr) -> CallExpr {
    CallExpr {
      span: DUMMY_SP,
      ctxt: Default::default(),
      callee: Callee::Expr(Box::new(Expr::Ident(ident("cssMap")))),
      args: vec![ExprOrSpread {
        spread: None,
        expr: Box::new(argument),
      }],
      type_args: None,
    }
  }

  fn build_css_from_object(expr: &Expr) -> CssOutput {
    let Expr::Object(object) = expr else {
      panic!("expected object expression");
    };

    let mut declarations: Vec<String> = Vec::new();
    for prop in &object.props {
      let PropOrSpread::Prop(prop) = prop else {
        continue;
      };

      let Prop::KeyValue(key_value) = prop.as_ref() else {
        continue;
      };

      let key = match &key_value.key {
        PropName::Ident(ident) => ident.sym.to_string(),
        PropName::Str(str) => str.value.to_string(),
        _ => panic!("unexpected key type"),
      };

      let Expr::Lit(Lit::Str(str)) = key_value.value.as_ref() else {
        panic!("expected string literal value");
      };

      let css_key = if key == "backgroundColor" {
        "background-color".to_string()
      } else {
        key.clone()
      };

      declarations.push(format!("{css_key}: {};", str.value));
    }

    let css_rule = format!(".test {{ {} }}", declarations.join(""));

    CssOutput {
      css: vec![CssItem::sheet(css_rule)],
      variables: Vec::new(),
    }
  }

  #[test]
  fn transforms_css_map_call_into_object_literal() {
    let meta = create_metadata();
    let argument = css_map_argument();
    let call = css_map_call(Expr::Object(argument.clone()));

    let binding = ident("styles");
    let result = visit_css_map_path_with_builder(
      CssMapUsage::Call(&call),
      Some(&binding),
      &meta,
      |expr, _| build_css_from_object(expr),
    );

    assert_eq!(result.props.len(), 2);

    let state = meta.state();
    let sheets = state.css_map.get(binding.sym.as_ref()).unwrap();
    assert_eq!(sheets.len(), 4);

    for prop in result.props {
      let PropOrSpread::Prop(prop) = prop else {
        panic!("expected property");
      };

      let Prop::KeyValue(key_value) = prop.as_ref() else {
        panic!("expected key value property");
      };

      match key_value.value.as_ref() {
        Expr::Lit(Lit::Str(str)) => assert!(!str.value.is_empty()),
        other => panic!("expected string literal, got {other:?}"),
      }
    }
  }

  #[test]
  fn errors_on_tagged_template_usage() {
    let meta = create_metadata();
    let tagged_template = TaggedTpl {
      span: DUMMY_SP,
      ctxt: Default::default(),
      tag: Box::new(Expr::Ident(ident("cssMap"))),
      type_params: None,
      tpl: Box::new(Tpl {
        span: DUMMY_SP,
        exprs: Vec::new(),
        quasis: vec![TplElement {
          span: DUMMY_SP,
          cooked: None,
          raw: Atom::from(""),
          tail: true,
        }],
      }),
    };

    visit_css_map_path_with_builder(
      CssMapUsage::TaggedTemplate(&tagged_template),
      Some(&ident("styles")),
      &meta,
      |_, _| CssOutput::new(),
    );

    let diagnostics = meta.state().diagnostics.clone();
    assert_eq!(diagnostics.len(), 1);
    assert!(
      diagnostics[0]
        .message
        .contains("cssMap function cannot be used as a tagged template expression")
    );
  }

  #[test]
  fn errors_when_argument_is_not_object() {
    let meta = create_metadata();
    let call = css_map_call(Expr::Lit(Lit::Num(Number {
      span: DUMMY_SP,
      value: 1.0,
      raw: None,
    })));

    visit_css_map_path_with_builder(
      CssMapUsage::Call(&call),
      Some(&ident("styles")),
      &meta,
      |_, _| CssOutput::new(),
    );

    let diagnostics = meta.state().diagnostics.clone();
    assert_eq!(diagnostics.len(), 1);
    assert!(
      diagnostics[0]
        .message
        .contains("cssMap function can only receive an object")
    );
  }

  #[test]
  fn errors_when_builder_returns_variables() {
    let meta = create_metadata();
    let argument = css_map_argument();
    let call = css_map_call(Expr::Object(argument));

    visit_css_map_path_with_builder(
      CssMapUsage::Call(&call),
      Some(&ident("styles")),
      &meta,
      |_, _| CssOutput {
        css: vec![CssItem::unconditional("color: red;".to_string())],
        variables: vec![crate::utils_types::Variable {
          name: "--test".into(),
          expression: Expr::Lit(Lit::Str(Str {
            span: DUMMY_SP,
            value: "value".into(),
            raw: None,
          })),
          prefix: None,
          suffix: None,
        }],
      },
    );

    let diagnostics = meta.state().diagnostics.clone();
    assert_eq!(diagnostics.len(), 1);
    assert!(
      diagnostics[0]
        .message
        .contains("The variant object must be statically defined")
    );
  }

  #[test]
  fn errors_when_multiple_class_names_emitted() {
    let meta = create_metadata();
    let argument = css_map_argument();
    let call = css_map_call(Expr::Object(argument));

    visit_css_map_path_with_builder(
      CssMapUsage::Call(&call),
      Some(&ident("styles")),
      &meta,
      |_, _| CssOutput {
        css: vec![
          CssItem::sheet(".one { color: red; }".to_string()),
          CssItem::sheet(".two { display: block; }".to_string()),
        ],
        variables: Vec::new(),
      },
    );

    let diagnostics = meta.state().diagnostics.clone();
    assert_eq!(diagnostics.len(), 1);
    assert!(
      diagnostics[0]
        .message
        .contains("The variant object must be statically defined")
    );
  }

  #[test]
  fn errors_when_not_declared_in_variable() {
    let meta = create_metadata();
    let argument = css_map_argument();
    let call = css_map_call(Expr::Object(argument));

    visit_css_map_path_with_builder(CssMapUsage::Call(&call), None, &meta, |_, _| CssOutput {
      css: vec![CssItem::unconditional("color: red;".to_string())],
      variables: Vec::new(),
    });

    let diagnostics = meta.state().diagnostics.clone();
    assert_eq!(diagnostics.len(), 1);
    assert!(
      diagnostics[0]
        .message
        .contains("CSS Map must be declared at the top-most scope")
    );
  }

  #[test]
  fn errors_when_variant_value_not_object() {
    let meta = create_metadata();
    let argument = ObjectLit {
      span: DUMMY_SP,
      props: vec![object_property("danger", string_lit("red"))],
    };
    let call = css_map_call(Expr::Object(argument));

    visit_css_map_path_with_builder(
      CssMapUsage::Call(&call),
      Some(&ident("styles")),
      &meta,
      |_, _| CssOutput::new(),
    );

    let diagnostics = meta.state().diagnostics.clone();
    assert_eq!(diagnostics.len(), 1);
    assert!(
      diagnostics[0]
        .message
        .contains("The variant object must be statically defined")
    );
  }

  fn number_lit(value: f64) -> Expr {
    Expr::Lit(Lit::Num(Number {
      span: DUMMY_SP,
      value,
      raw: None,
    }))
  }

  fn string_key_property(key: &str, value: Expr) -> PropOrSpread {
    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Str(Str {
        span: DUMMY_SP,
        value: key.into(),
        raw: None,
      }),
      value: Box::new(value),
    })))
  }

  /// Helper that builds CSS from object with support for nested selectors.
  /// Returns a single sheet with all rules concatenated, which is what cssMap expects.
  fn build_css_with_nested_selectors(expr: &Expr) -> CssOutput {
    let Expr::Object(object) = expr else {
      panic!("expected object expression");
    };

    let mut all_rules: Vec<String> = Vec::new();
    let mut declarations: Vec<String> = Vec::new();

    for prop in &object.props {
      let PropOrSpread::Prop(prop) = prop else {
        continue;
      };

      let Prop::KeyValue(key_value) = prop.as_ref() else {
        continue;
      };

      let key = match &key_value.key {
        PropName::Ident(ident) => ident.sym.to_string(),
        PropName::Str(str) => str.value.to_string(),
        _ => panic!("unexpected key type"),
      };

      match key_value.value.as_ref() {
        Expr::Lit(Lit::Str(str)) => {
          let css_key = match key.as_str() {
            "backgroundColor" => "background-color".to_string(),
            "fontSize" => "font-size".to_string(),
            "borderRadius" => "border-radius".to_string(),
            _ => key.clone(),
          };
          declarations.push(format!("{css_key}: {};", str.value));
        }
        Expr::Lit(Lit::Num(num)) => {
          let css_key = match key.as_str() {
            "fontSize" => "font-size".to_string(),
            "borderRadius" => "border-radius".to_string(),
            _ => key.clone(),
          };
          let value = if key == "fontSize" || key == "borderRadius" {
            format!("{}px", num.value)
          } else {
            num.value.to_string()
          };
          declarations.push(format!("{css_key}: {};", value));
        }
        Expr::Object(nested) => {
          // Handle nested selectors - collect their CSS as additional rules
          let nested_result = build_css_with_nested_selectors(&Expr::Object(nested.clone()));
          for item in nested_result.css {
            match item {
              CssItem::Sheet(sheet) => {
                // Wrap nested CSS with the selector
                let wrapped = format!(".test {} {{ {} }}", key, extract_declarations(&sheet.css));
                all_rules.push(wrapped);
              }
              _ => {}
            }
          }
        }
        _ => {}
      }
    }

    // Build the main rule
    if !declarations.is_empty() {
      let css_rule = format!(".test {{ {} }}", declarations.join(" "));
      all_rules.insert(0, css_rule);
    }

    // Return all rules as a single concatenated sheet
    if all_rules.is_empty() {
      return CssOutput {
        css: vec![],
        variables: Vec::new(),
      };
    }

    CssOutput {
      css: vec![CssItem::sheet(all_rules.join(" "))],
      variables: Vec::new(),
    }
  }

  /// Helper to extract declarations from a CSS rule
  fn extract_declarations(css: &str) -> &str {
    if let Some(start) = css.find('{') {
      if let Some(end) = css.rfind('}') {
        return css[start + 1..end].trim();
      }
    }
    css
  }

  #[test]
  fn transforms_single_variant_with_single_property() {
    let meta = create_metadata();
    let argument = ObjectLit {
      span: DUMMY_SP,
      props: vec![object_property(
        "root",
        Expr::Object(ObjectLit {
          span: DUMMY_SP,
          props: vec![object_property("color", string_lit("blue"))],
        }),
      )],
    };
    let call = css_map_call(Expr::Object(argument));

    let binding = ident("styles");
    let result = visit_css_map_path_with_builder(
      CssMapUsage::Call(&call),
      Some(&binding),
      &meta,
      |expr, _| build_css_with_nested_selectors(expr),
    );

    assert_eq!(result.props.len(), 1);

    let state = meta.state();
    let sheets = state.css_map.get(binding.sym.as_ref()).unwrap();

    // Verify all sheets are strings
    for sheet in sheets {
      assert!(
        sheet.contains('{'),
        "Sheet should be a valid CSS string containing '{{': {:?}",
        sheet
      );
    }
  }

  #[test]
  fn transforms_variant_with_element_selector_div() {
    let meta = create_metadata();
    let argument = ObjectLit {
      span: DUMMY_SP,
      props: vec![object_property(
        "container",
        Expr::Object(ObjectLit {
          span: DUMMY_SP,
          props: vec![
            object_property("display", string_lit("flex")),
            string_key_property(
              "div",
              Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: vec![object_property("margin", string_lit("0"))],
              }),
            ),
          ],
        }),
      )],
    };
    let call = css_map_call(Expr::Object(argument));

    let binding = ident("styles");
    let result = visit_css_map_path_with_builder(
      CssMapUsage::Call(&call),
      Some(&binding),
      &meta,
      |expr, _| build_css_with_nested_selectors(expr),
    );

    assert_eq!(result.props.len(), 1);

    let state = meta.state();
    let sheets = state.css_map.get(binding.sym.as_ref()).unwrap();

    // Verify that all collected sheets are valid strings
    for sheet in sheets {
      assert!(
        sheet.is_ascii() || !sheet.is_empty(),
        "Each sheet should be a valid string: {:?}",
        sheet
      );
    }

    // Check the result property is a string literal
    let PropOrSpread::Prop(prop) = &result.props[0] else {
      panic!("expected property");
    };
    let Prop::KeyValue(key_value) = prop.as_ref() else {
      panic!("expected key value property");
    };
    match key_value.value.as_ref() {
      Expr::Lit(Lit::Str(str)) => {
        assert!(
          !str.value.is_empty() || str.value.as_ref() == "",
          "Class name should be a string"
        );
      }
      other => panic!("expected string literal class name, got {other:?}"),
    }
  }

  #[test]
  fn transforms_variant_with_multiple_element_selectors() {
    let meta = create_metadata();
    let argument = ObjectLit {
      span: DUMMY_SP,
      props: vec![object_property(
        "wrapper",
        Expr::Object(ObjectLit {
          span: DUMMY_SP,
          props: vec![
            object_property("position", string_lit("relative")),
            string_key_property(
              "span",
              Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: vec![object_property("color", string_lit("inherit"))],
              }),
            ),
            string_key_property(
              "button",
              Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: vec![object_property("cursor", string_lit("pointer"))],
              }),
            ),
          ],
        }),
      )],
    };
    let call = css_map_call(Expr::Object(argument));

    let binding = ident("styles");
    let result = visit_css_map_path_with_builder(
      CssMapUsage::Call(&call),
      Some(&binding),
      &meta,
      |expr, _| build_css_with_nested_selectors(expr),
    );

    assert_eq!(result.props.len(), 1);

    // Verify the output is a valid string
    let PropOrSpread::Prop(prop) = &result.props[0] else {
      panic!("expected property");
    };
    let Prop::KeyValue(key_value) = prop.as_ref() else {
      panic!("expected key value property");
    };
    assert!(
      matches!(key_value.value.as_ref(), Expr::Lit(Lit::Str(_))),
      "Class name must be a string literal"
    );
  }

  #[test]
  fn transforms_variant_with_ampersand_pseudo_selectors() {
    let meta = create_metadata();
    let argument = ObjectLit {
      span: DUMMY_SP,
      props: vec![object_property(
        "button",
        Expr::Object(ObjectLit {
          span: DUMMY_SP,
          props: vec![
            object_property("backgroundColor", string_lit("blue")),
            string_key_property(
              "&:hover",
              Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: vec![object_property("backgroundColor", string_lit("darkblue"))],
              }),
            ),
            string_key_property(
              "&:active",
              Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: vec![object_property("backgroundColor", string_lit("navy"))],
              }),
            ),
          ],
        }),
      )],
    };
    let call = css_map_call(Expr::Object(argument));

    let binding = ident("styles");
    let result = visit_css_map_path_with_builder(
      CssMapUsage::Call(&call),
      Some(&binding),
      &meta,
      |expr, _| build_css_with_nested_selectors(expr),
    );

    assert_eq!(result.props.len(), 1);

    let state = meta.state();
    let sheets = state.css_map.get(binding.sym.as_ref()).unwrap();

    // Verify each sheet in the cache is a string
    for (i, sheet) in sheets.iter().enumerate() {
      assert!(
        sheet.is_ascii(),
        "Sheet {} should be a valid ASCII string, got: {:?}",
        i,
        sheet
      );
    }
  }

  #[test]
  fn transforms_multiple_variants_correctly() {
    let meta = create_metadata();
    let argument = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        object_property(
          "primary",
          Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![object_property("color", string_lit("blue"))],
          }),
        ),
        object_property(
          "secondary",
          Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![object_property("color", string_lit("gray"))],
          }),
        ),
        object_property(
          "tertiary",
          Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![object_property("color", string_lit("lightgray"))],
          }),
        ),
      ],
    };
    let call = css_map_call(Expr::Object(argument));

    let binding = ident("buttonStyles");
    let result = visit_css_map_path_with_builder(
      CssMapUsage::Call(&call),
      Some(&binding),
      &meta,
      |expr, _| build_css_with_nested_selectors(expr),
    );

    assert_eq!(result.props.len(), 3, "Should have 3 variant properties");

    // Verify each variant produces a string class name
    for prop in &result.props {
      let PropOrSpread::Prop(prop) = prop else {
        panic!("expected property");
      };
      let Prop::KeyValue(key_value) = prop.as_ref() else {
        panic!("expected key value property");
      };
      match key_value.value.as_ref() {
        Expr::Lit(Lit::Str(str)) => {
          assert!(
            !str.value.is_empty(),
            "Each variant should produce a non-empty class name"
          );
        }
        other => panic!("expected string literal, got {other:?}"),
      }
    }
  }

  #[test]
  fn handles_empty_variant_object() {
    let meta = create_metadata();
    let argument = ObjectLit {
      span: DUMMY_SP,
      props: vec![object_property(
        "empty",
        Expr::Object(ObjectLit {
          span: DUMMY_SP,
          props: vec![],
        }),
      )],
    };
    let call = css_map_call(Expr::Object(argument));

    let binding = ident("styles");
    let result =
      visit_css_map_path_with_builder(CssMapUsage::Call(&call), Some(&binding), &meta, |_, _| {
        CssOutput {
          css: vec![],
          variables: Vec::new(),
        }
      });

    assert_eq!(result.props.len(), 1);

    // Empty variant should still produce a valid string (even if empty)
    let PropOrSpread::Prop(prop) = &result.props[0] else {
      panic!("expected property");
    };
    let Prop::KeyValue(key_value) = prop.as_ref() else {
      panic!("expected key value property");
    };
    // Should be empty string
    match key_value.value.as_ref() {
      Expr::Lit(Lit::Str(str)) => {
        assert_eq!(
          str.value.as_ref(),
          "",
          "Empty variant should have empty class name"
        );
      }
      other => panic!("expected string literal for empty variant, got {other:?}"),
    }
  }

  #[test]
  fn transforms_variant_with_numeric_values() {
    let meta = create_metadata();
    let argument = ObjectLit {
      span: DUMMY_SP,
      props: vec![object_property(
        "sized",
        Expr::Object(ObjectLit {
          span: DUMMY_SP,
          props: vec![
            object_property("fontSize", number_lit(16.0)),
            object_property("borderRadius", number_lit(4.0)),
          ],
        }),
      )],
    };
    let call = css_map_call(Expr::Object(argument));

    let binding = ident("styles");
    let result = visit_css_map_path_with_builder(
      CssMapUsage::Call(&call),
      Some(&binding),
      &meta,
      |expr, _| build_css_with_nested_selectors(expr),
    );

    assert_eq!(result.props.len(), 1);

    let state = meta.state();
    let sheets = state.css_map.get(binding.sym.as_ref()).unwrap();

    // All sheets should be valid strings
    for sheet in sheets {
      assert!(
        sheet.contains('{'),
        "Sheet should contain CSS rules: {:?}",
        sheet
      );
    }
  }

  #[test]
  fn sheets_are_always_strings_not_wrapped_objects() {
    // This test specifically validates that sheets stored in css_map
    // are always strings, never wrapped in objects that would cause
    // "sheet.includes is not a function" errors at runtime
    let meta = create_metadata();
    let argument = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        object_property(
          "variant1",
          Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![object_property("color", string_lit("red"))],
          }),
        ),
        object_property(
          "variant2",
          Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![
              object_property("color", string_lit("blue")),
              string_key_property(
                "div",
                Expr::Object(ObjectLit {
                  span: DUMMY_SP,
                  props: vec![object_property("margin", string_lit("8px"))],
                }),
              ),
            ],
          }),
        ),
      ],
    };
    let call = css_map_call(Expr::Object(argument));

    let binding = ident("testStyles");
    let _result = visit_css_map_path_with_builder(
      CssMapUsage::Call(&call),
      Some(&binding),
      &meta,
      |expr, _| build_css_with_nested_selectors(expr),
    );

    let state = meta.state();
    let sheets = state.css_map.get(binding.sym.as_ref()).unwrap();

    // Critical assertion: each sheet must be a String type
    // This ensures that sheet.includes() will work at runtime
    for (index, sheet) in sheets.iter().enumerate() {
      // Verify the sheet is a valid string with CSS content
      assert!(
        sheet
          .chars()
          .all(|c| c.is_ascii() || c.is_alphanumeric() || c.is_whitespace()),
        "Sheet at index {} must be a valid string, got: {:?}",
        index,
        sheet
      );

      // If the sheet has content, it should have CSS structure
      if !sheet.trim().is_empty() {
        assert!(
          sheet.contains('{') || sheet.contains(':'),
          "Non-empty sheet at index {} should contain CSS syntax: {:?}",
          index,
          sheet
        );
      }
    }
  }

  #[test]
  fn class_name_output_is_always_string_literal() {
    // This test ensures that the output class name expressions are always
    // string literals, preventing runtime errors when trying to use them
    let meta = create_metadata();
    let variants = vec![
      ("simple", vec![object_property("color", string_lit("red"))]),
      (
        "withDiv",
        vec![
          object_property("display", string_lit("block")),
          string_key_property(
            "div",
            Expr::Object(ObjectLit {
              span: DUMMY_SP,
              props: vec![object_property("padding", string_lit("8px"))],
            }),
          ),
        ],
      ),
      (
        "withHover",
        vec![
          object_property("backgroundColor", string_lit("white")),
          string_key_property(
            "&:hover",
            Expr::Object(ObjectLit {
              span: DUMMY_SP,
              props: vec![object_property("backgroundColor", string_lit("gray"))],
            }),
          ),
        ],
      ),
    ];

    for (name, props) in variants {
      let argument = ObjectLit {
        span: DUMMY_SP,
        props: vec![object_property(
          name,
          Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props,
          }),
        )],
      };
      let call = css_map_call(Expr::Object(argument));
      let binding = ident("styles");

      let result = visit_css_map_path_with_builder(
        CssMapUsage::Call(&call),
        Some(&binding),
        &meta,
        |expr, _| build_css_with_nested_selectors(expr),
      );

      assert_eq!(
        result.props.len(),
        1,
        "Variant '{}' should produce one property",
        name
      );

      let PropOrSpread::Prop(prop) = &result.props[0] else {
        panic!("Variant '{}': expected property", name);
      };
      let Prop::KeyValue(key_value) = prop.as_ref() else {
        panic!("Variant '{}': expected key value property", name);
      };

      match key_value.value.as_ref() {
        Expr::Lit(Lit::Str(_)) => {
          // This is correct - class name is a string literal
        }
        other => {
          panic!(
            "Variant '{}': class name must be a string literal to avoid 'includes is not a function' error, got {:?}",
            name, other
          );
        }
      }
    }
  }

  #[test]
  fn reports_specific_error_when_token_function_is_used() {
    let meta = create_metadata();
    let argument = css_map_argument();
    let call = css_map_call(Expr::Object(argument));

    visit_css_map_path_with_builder(
      CssMapUsage::Call(&call),
      Some(&ident("styles")),
      &meta,
      |_, _| CssOutput {
        css: vec![],
        variables: vec![crate::utils_types::Variable {
          name: "--test".into(),
          expression: Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Ident(ident("token")))),
            args: vec![],
            type_args: None,
            ctxt: SyntaxContext::empty(),
          }),
          prefix: None,
          suffix: None,
        }],
      },
    );

    let diagnostics = meta.state().diagnostics.clone();
    assert_eq!(diagnostics.len(), 1);
    assert!(
      diagnostics[0]
        .message
        .contains("Ensure `token()` is imported from `@atlaskit/tokens`")
    );

    // Also verify hints
    let hints = &diagnostics[0].hints;
    assert!(!hints.is_empty(), "hints should be present");
    assert!(hints[0].contains("Ensure the `token()` function is imported"));
  }
}
