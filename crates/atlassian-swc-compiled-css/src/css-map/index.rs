use swc_core::atoms::Atom;
use swc_core::common::{DUMMY_SP, Span, Spanned};
use swc_core::ecma::ast::{
  CallExpr, Expr, Ident, KeyValueProp, Lit, ObjectLit, Prop, PropOrSpread, Str, TaggedTpl,
};

use crate::css_map_process_selectors::merge_extended_selectors_into_properties;
use crate::types::Metadata;
use crate::utils_css_builders::build_css as build_css_from_expr;
use crate::utils_css_map::{
  ErrorMessages, error_if_not_valid_object_property, report_css_map_error,
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
      report_css_map_error(
        meta,
        tagged_tpl.span,
        ErrorMessages::NoTaggedTemplate.message(),
      );
      return empty_object(tagged_tpl.span);
    }
    CssMapUsage::Call(call_expr) => {
      let Some(binding_identifier) = parent_identifier else {
        report_css_map_error(meta, call_expr.span, ErrorMessages::DefineMap.message());
        return empty_object(call_expr.span);
      };

      if call_expr.args.len() != 1 {
        report_css_map_error(
          meta,
          call_expr.span,
          ErrorMessages::NumberOfArgument.message(),
        );
        return empty_object(call_expr.span);
      }

      let argument = &call_expr.args[0];
      if argument.spread.is_some() {
        report_css_map_error(meta, argument.span(), ErrorMessages::ArgumentType.message());
        return empty_object(call_expr.span);
      }

      let Expr::Object(object_lit) = argument.expr.as_ref() else {
        report_css_map_error(
          meta,
          argument.expr.span(),
          ErrorMessages::ArgumentType.message(),
        );
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
          report_css_map_error(
            meta,
            key_value.value.span(),
            ErrorMessages::StaticVariantObject.message(),
          );
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
          report_css_map_error(
            meta,
            key_value.value.span(),
            ErrorMessages::StaticVariantObject.message(),
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
  use std::panic::AssertUnwindSafe;
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

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
      visit_css_map_path_with_builder(
        CssMapUsage::TaggedTemplate(&tagged_template),
        Some(&ident("styles")),
        &meta,
        |_, _| CssOutput::new(),
      );
    }));

    let message = match result {
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

    assert!(message.contains("cssMap function cannot be used as a tagged template expression"));
  }

  #[test]
  fn errors_when_argument_is_not_object() {
    let meta = create_metadata();
    let call = css_map_call(Expr::Lit(Lit::Num(Number {
      span: DUMMY_SP,
      value: 1.0,
      raw: None,
    })));

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
      visit_css_map_path_with_builder(
        CssMapUsage::Call(&call),
        Some(&ident("styles")),
        &meta,
        |_, _| CssOutput::new(),
      );
    }));

    let message = match result {
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

    assert!(message.contains("cssMap function can only receive an object"));
  }

  #[test]
  fn errors_when_builder_returns_variables() {
    let meta = create_metadata();
    let argument = css_map_argument();
    let call = css_map_call(Expr::Object(argument));

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
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
    }));

    let message = match result {
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

    assert!(message.contains("The variant object must be statically defined"));
  }

  #[test]
  fn errors_when_multiple_class_names_emitted() {
    let meta = create_metadata();
    let argument = css_map_argument();
    let call = css_map_call(Expr::Object(argument));

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
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
    }));

    let message = match result {
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

    assert!(message.contains("The variant object must be statically defined"));
  }

  #[test]
  fn errors_when_not_declared_in_variable() {
    let meta = create_metadata();
    let argument = css_map_argument();
    let call = css_map_call(Expr::Object(argument));

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
      visit_css_map_path_with_builder(CssMapUsage::Call(&call), None, &meta, |_, _| CssOutput {
        css: vec![CssItem::unconditional("color: red;".to_string())],
        variables: Vec::new(),
      });
    }));

    let message = match result {
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

    assert!(message.contains("CSS Map must be declared at the top-most scope"));
  }

  #[test]
  fn errors_when_variant_value_not_object() {
    let meta = create_metadata();
    let argument = ObjectLit {
      span: DUMMY_SP,
      props: vec![object_property("danger", string_lit("red"))],
    };
    let call = css_map_call(Expr::Object(argument));

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
      visit_css_map_path_with_builder(
        CssMapUsage::Call(&call),
        Some(&ident("styles")),
        &meta,
        |_, _| CssOutput::new(),
      );
    }));

    let message = match result {
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

    assert!(message.contains("The variant object must be statically defined"));
  }
}
