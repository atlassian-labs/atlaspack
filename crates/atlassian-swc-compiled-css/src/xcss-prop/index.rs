use swc_core::common::{DUMMY_SP, Spanned, SyntaxContext};
use swc_core::ecma::ast::{
  Expr, Ident, JSXAttr, JSXAttrName, JSXAttrOrSpread, JSXAttrValue, JSXElement, JSXElementChild,
  JSXExpr, JSXExprContainer, MemberExpr, MemberProp, ObjectLit, Prop, PropOrSpread,
};
use swc_core::ecma::visit::{Visit, VisitWith, noop_visit_type};

use crate::postcss::plugins::extract_stylesheets::normalize_block_value_spacing;
use crate::types::Metadata;

use crate::utils_build_compiled_component::compiled_template;
use crate::utils_css_builders::{
  build_css as build_css_from_expr, generate_cache_for_css_map_with_builder,
};
use crate::utils_transform_css_items::transform_css_items;
use crate::utils_types::CssOutput;

fn is_xcss_attribute(attr: &JSXAttr) -> bool {
  match &attr.name {
    JSXAttrName::Ident(ident) => ident.sym.as_ref().to_lowercase().ends_with("xcss"),
    _ => false,
  }
}

fn get_expression_container(attr: &mut JSXAttr) -> Option<&mut JSXExprContainer> {
  match attr.value.as_mut()? {
    JSXAttrValue::JSXExprContainer(container) => Some(container),
    _ => None,
  }
}

fn undefined_expr() -> Expr {
  Expr::Ident(Ident::new(
    "undefined".into(),
    DUMMY_SP,
    SyntaxContext::empty(),
  ))
}

fn expression_is_static(expr: &Expr) -> bool {
  use swc_core::ecma::ast::{
    ArrayLit, BinExpr, CondExpr, ExprOrSpread, Lit, OptChainExpr, ParenExpr, UnaryExpr,
  };

  match expr {
    Expr::Lit(Lit::Str(_))
    | Expr::Lit(Lit::Num(_))
    | Expr::Lit(Lit::Bool(_))
    | Expr::Lit(Lit::Null(_))
    | Expr::Lit(Lit::BigInt(_))
    | Expr::Lit(Lit::Regex(_)) => true,
    Expr::Tpl(tpl) => tpl.exprs.is_empty(),
    Expr::Ident(ident) => matches!(ident.sym.as_ref(), "undefined" | "NaN" | "Infinity"),
    Expr::Array(ArrayLit { elems, .. }) => elems.iter().all(|elem| match elem {
      Some(ExprOrSpread {
        spread: None, expr, ..
      }) => expression_is_static(expr),
      _ => false,
    }),
    Expr::Object(ObjectLit { props, .. }) => props.iter().all(|prop| match prop {
      PropOrSpread::Prop(prop) => match &**prop {
        Prop::KeyValue(kv) => expression_is_static(&kv.value),
        Prop::Assign(assign) => expression_is_static(&assign.value),
        Prop::Getter(_) | Prop::Setter(_) | Prop::Method(_) | Prop::Shorthand(_) => false,
      },
      PropOrSpread::Spread(_) => false,
    }),
    Expr::Unary(UnaryExpr { arg, .. }) => expression_is_static(arg),
    Expr::Bin(BinExpr { left, right, .. }) => {
      expression_is_static(left) && expression_is_static(right)
    }
    Expr::Cond(CondExpr {
      test, cons, alt, ..
    }) => expression_is_static(test) && expression_is_static(cons) && expression_is_static(alt),
    Expr::Paren(ParenExpr { expr, .. }) => expression_is_static(expr),
    Expr::Seq(seq) => seq.exprs.iter().all(|expr| expression_is_static(expr)),
    Expr::TsAs(ts_as) => expression_is_static(&ts_as.expr),
    Expr::TsTypeAssertion(assertion) => expression_is_static(&assertion.expr),
    Expr::TsConstAssertion(assertion) => expression_is_static(&assertion.expr),
    Expr::TsNonNull(non_null) => expression_is_static(&non_null.expr),
    Expr::TsInstantiation(inst) => expression_is_static(&inst.expr),
    Expr::TsSatisfies(sat) => expression_is_static(&sat.expr),
    Expr::OptChain(OptChainExpr { .. }) => false,
    Expr::Call(_)
    | Expr::Member(_)
    | Expr::New(_)
    | Expr::Await(_)
    | Expr::Yield(_)
    | Expr::Assign(_)
    | Expr::Update(_)
    | Expr::MetaProp(_)
    | Expr::Arrow(_)
    | Expr::Fn(_)
    | Expr::Class(_)
    | Expr::TaggedTpl(_)
    | Expr::JSXElement(_)
    | Expr::JSXFragment(_)
    | Expr::PrivateName(_)
    | Expr::SuperProp(_)
    | Expr::This(_) => false,
    Expr::Invalid(_) => false,
    _ => false,
  }
}

fn static_object_invariant(expr: &Expr, _meta: &Metadata) {
  if !expression_is_static(expr) {
    panic!("Object given to the xcss prop must be static.");
  }
}

fn collect_pass_styles(meta: &Metadata, identifiers: &[String]) -> Vec<String> {
  let state = meta.state();
  let mut styles = Vec::new();

  for (key, sheets) in state.css_map.iter() {
    if identifiers.iter().any(|identifier| identifier == key) {
      styles.extend(sheets.clone());
    }
  }

  styles
}

fn collect_member_expression_identifiers(expr: &Expr) -> Vec<Ident> {
  struct Collector {
    identifiers: Vec<Ident>,
  }

  impl Visit for Collector {
    noop_visit_type!();

    fn visit_member_expr(&mut self, member: &MemberExpr) {
      if let Expr::Ident(object_ident) = &*member.obj {
        self.identifiers.push(object_ident.clone());
      }

      member.obj.visit_with(self);

      if let MemberProp::Computed(computed) = &member.prop {
        computed.expr.visit_with(self);
      }
    }

    fn visit_opt_chain_expr(&mut self, expr: &swc_core::ecma::ast::OptChainExpr) {
      expr.visit_children_with(self);
    }
  }

  let mut collector = Collector {
    identifiers: Vec::new(),
  };
  expr.visit_with(&mut collector);
  collector.identifiers
}

fn record_style_rules_from_sheets(sheets: &[String], meta: &Metadata) {
  if sheets.is_empty() {
    return;
  }

  let should_collect = meta.state().opts.extract.unwrap_or(false);
  if !should_collect {
    return;
  }

  let mut state = meta.state_mut();
  for sheet in sheets {
    if !sheet.contains('{') {
      continue;
    }

    let normalized = normalize_block_value_spacing(sheet);
    state.style_rules.insert(normalized);
  }
}

fn find_inner_component(element: &JSXElement) -> Option<&JSXElement> {
  for child in &element.children {
    if let JSXElementChild::JSXExprContainer(container) = child {
      if let JSXExpr::Expr(expr) = &container.expr {
        if let Expr::JSXElement(inner) = &**expr {
          return Some(inner);
        }
      }
    }
  }

  None
}

/// Mirrors the Babel `visitXcssPropPath` helper. The caller injects the CSS builder so tests can
/// provide precomputed output until the native `build_css` port is available.
pub fn visit_xcss_prop_with_builder<F>(node: &mut Expr, meta: &Metadata, build_css: &mut F) -> bool
where
  F: FnMut(&Expr, &Metadata) -> CssOutput,
{
  let Expr::JSXElement(element) = node else {
    return false;
  };

  {
    let mut state = meta.state_mut();
    if state.transform_cache.has(element.opening.span) {
      return false;
    }
    state.transform_cache.set(element.opening.span);
  }

  let attr_index = element
    .opening
    .attrs
    .iter()
    .enumerate()
    .find_map(|(index, attr)| match attr {
      JSXAttrOrSpread::JSXAttr(attr) if is_xcss_attribute(attr) => Some(index),
      _ => None,
    });

  let Some(index) = attr_index else {
    return false;
  };

  let JSXAttrOrSpread::JSXAttr(attr) = &mut element.opening.attrs[index] else {
    return false;
  };

  let Some(container) = get_expression_container(attr) else {
    return false;
  };

  match &mut container.expr {
    JSXExpr::JSXEmptyExpr(_) => return false,
    JSXExpr::Expr(expr) => match expr.as_mut() {
      Expr::Object(object) => {
        let object_expr = Expr::Object(object.clone());
        static_object_invariant(&object_expr, meta);

        let CssOutput { css, .. } = build_css(&object_expr, meta);
        let crate::utils_transform_css_items::TransformCssItemsResult {
          sheets,
          class_names,
        } = transform_css_items(&css, meta);
        let mut class_iter = class_names.into_iter();

        let replacement = match (class_iter.next(), class_iter.next()) {
          (Some(class_name), None) => class_name,
          (None, _) => undefined_expr(),
          _ => {
            panic!("Unexpected count of class names please raise an issue on Github.");
          }
        };

        **expr = replacement;

        meta.state_mut().uses_xcss = true;

        let inner = find_inner_component(element)
          .map(|inner| inner.clone())
          .unwrap_or_else(|| (**element).clone());

        record_style_rules_from_sheets(&sheets, meta);
        *node = compiled_template(Expr::JSXElement(Box::new(inner)), &sheets, meta);

        true
      }
      expression => {
        let identifiers = collect_member_expression_identifiers(expression);

        for identifier in &identifiers {
          generate_cache_for_css_map_with_builder(identifier, meta, build_css);
        }

        let identifier_names: Vec<String> = identifiers
          .iter()
          .map(|ident| ident.sym.as_ref().to_string())
          .collect();
        let sheets = collect_pass_styles(meta, &identifier_names);

        if sheets.is_empty() {
          return false;
        }

        meta.state_mut().uses_xcss = true;

        let inner = find_inner_component(element)
          .map(|inner| inner.clone())
          .unwrap_or_else(|| (**element).clone());

        record_style_rules_from_sheets(&sheets, meta);
        *node = compiled_template(Expr::JSXElement(Box::new(inner)), &sheets, meta);

        true
      }
    },
  }
}

/// Wrapper that uses the shared `build_css` helper to transform `xcss` props
/// without requiring callers to provide a custom builder.
pub fn visit_xcss_prop(node: &mut Expr, meta: &Metadata) -> bool {
  let mut build = |expr: &Expr, metadata: &Metadata| build_css_from_expr(expr, metadata);
  visit_xcss_prop_with_builder(node, meta, &mut build)
}

/// Applies the xcss transform directly to a `JSXElement`, allowing callers to
/// run the transform without wrapping the element in an `Expr`.
pub fn visit_xcss_prop_on_element(element: &mut JSXElement, meta: &Metadata) -> bool {
  let mut expr = Expr::JSXElement(Box::new(element.clone()));
  let meta_with_context = meta
    .with_parent_expr(Some(&expr))
    .with_own_span(Some(expr.span()));
  let transformed = visit_xcss_prop(&mut expr, &meta_with_context);
  if transformed {
    if let Expr::JSXElement(updated) = expr {
      *element = *updated;
    } else {
      panic!("xcss transform must yield JSXElement");
    }
  }

  transformed
}

#[cfg(test)]
mod tests {
  use super::visit_xcss_prop_with_builder;
  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};
  use crate::utils_types::{CssItem, CssOutput};
  use std::cell::RefCell;
  use std::panic::AssertUnwindSafe;
  use std::rc::Rc;
  use swc_core::common::comments::Comment;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{FileName, SourceMap};
  use swc_core::ecma::ast::{
    Expr, JSXAttrName, JSXAttrOrSpread, JSXAttrValue, JSXElementName, JSXExpr, Lit, Str,
  };
  use swc_core::ecma::parser::lexer::Lexer;
  use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax};

  fn parse_jsx_expression(
    code: &str,
    cm: Option<Lrc<SourceMap>>,
  ) -> (Expr, Lrc<SourceMap>, Lrc<swc_core::common::SourceFile>) {
    let cm: Lrc<SourceMap> = cm.unwrap_or_default();
    let fm = cm.new_source_file(FileName::Custom("expr.jsx".into()).into(), code.to_string());
    let lexer = Lexer::new(
      Syntax::Es(EsSyntax {
        jsx: true,
        ..Default::default()
      }),
      Default::default(),
      StringInput::from(&*fm),
      None,
    );

    let mut parser = Parser::new_from(lexer);
    let expr = parser.parse_expr().expect("parse JSX expression");

    (*expr, cm, fm)
  }

  fn create_metadata(cm: Lrc<SourceMap>, comments: Vec<Comment>) -> Metadata {
    let file = TransformFile::new(cm, comments);
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));

    Metadata::new(state)
  }

  fn find_inner_component<'a>(expr: &'a Expr) -> &'a swc_core::ecma::ast::JSXElement {
    let Expr::JSXElement(wrapper) = expr else {
      panic!("expected JSX element");
    };

    super::find_inner_component(wrapper).unwrap_or_else(|| panic!("inner component not found"))
  }

  #[test]
  fn transforms_inline_object_into_compiled_template() {
    let code = "<Component xcss={{ color: 'red' }} />";
    let (mut expr, cm, _fm) = parse_jsx_expression(code, None);
    let metadata = create_metadata(cm, Vec::new());

    let mut calls = 0usize;
    let mut build_css = |value: &Expr, _meta: &Metadata| {
      calls += 1;
      assert!(matches!(value, Expr::Object(_)));
      CssOutput {
        css: vec![CssItem::unconditional(String::from("._a{color:red;}"))],
        variables: Vec::new(),
      }
    };

    let transformed = visit_xcss_prop_with_builder(&mut expr, &metadata, &mut build_css);
    assert!(transformed);
    assert_eq!(calls, 1);

    let Expr::JSXElement(wrapper) = &expr else {
      panic!("expected JSX element wrapper");
    };

    match &wrapper.opening.name {
      JSXElementName::Ident(ident) => assert_eq!(ident.sym.as_ref(), "CC"),
      other => panic!("expected CC wrapper, found {other:?}"),
    }

    let inner = find_inner_component(&expr);
    let attr = inner
            .opening
            .attrs
            .iter()
            .find_map(|attr| match attr {
                JSXAttrOrSpread::JSXAttr(attr)
                    if matches!(&attr.name, JSXAttrName::Ident(ident) if ident.sym.as_ref() == "xcss") =>
                {
                    Some(attr)
                }
                _ => None,
            })
            .expect("xcss attribute");

    let JSXAttrValue::JSXExprContainer(container) = attr.value.as_ref().expect("attr value") else {
      panic!("expected expression container");
    };

    match &container.expr {
      JSXExpr::Expr(expr) => match &**expr {
        Expr::Lit(Lit::Str(Str { value, .. })) => assert!(!value.is_empty()),
        other => panic!("expected string literal, found {other:?}"),
      },
      other => panic!("expected expression, found {other:?}"),
    }

    assert!(metadata.state().uses_xcss);
  }

  #[test]
  fn throws_for_dynamic_inline_object() {
    let code = "<Component xcss={{ color: value }} />";
    let (mut expr, cm, _fm) = parse_jsx_expression(code, None);
    let metadata = create_metadata(cm, Vec::new());

    let mut build_css = |_value: &Expr, _meta: &Metadata| -> CssOutput {
      panic!("builder should not be invoked");
    };

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
      visit_xcss_prop_with_builder(&mut expr, &metadata, &mut build_css);
    }));

    let message = result.expect_err("expected panic");
    let error = message
      .downcast_ref::<String>()
      .cloned()
      .or_else(|| {
        message
          .downcast_ref::<&'static str>()
          .map(|s| s.to_string())
      })
      .unwrap();

    assert!(error.contains("Object given to the xcss prop must be static"));
  }

  #[test]
  fn transforms_css_map_usage_when_styles_available() {
    let code = "<Component xcss={styles.primary} />";
    let (mut expr, cm, _fm) = parse_jsx_expression(code, None);
    let metadata = create_metadata(cm, Vec::new());

    {
      let mut state = metadata.state_mut();
      state
        .css_map
        .insert("styles".into(), vec!["._hash{color:red}".into()]);
    }

    let mut calls = 0usize;
    let mut build_css = |_value: &Expr, _meta: &Metadata| {
      calls += 1;
      CssOutput::new()
    };

    let transformed = visit_xcss_prop_with_builder(&mut expr, &metadata, &mut build_css);
    assert!(transformed);
    assert_eq!(calls, 0);

    let Expr::JSXElement(wrapper) = &expr else {
      panic!("expected JSX element wrapper");
    };

    match &wrapper.opening.name {
      JSXElementName::Ident(ident) => assert_eq!(ident.sym.as_ref(), "CC"),
      other => panic!("expected CC wrapper, found {other:?}"),
    }

    let inner = find_inner_component(&expr);
    let attr = inner
            .opening
            .attrs
            .iter()
            .find_map(|attr| match attr {
                JSXAttrOrSpread::JSXAttr(attr)
                    if matches!(&attr.name, JSXAttrName::Ident(ident) if ident.sym.as_ref() == "xcss") =>
                {
                    Some(attr)
                }
                _ => None,
            })
            .expect("xcss attribute");

    let JSXAttrValue::JSXExprContainer(container) = attr.value.as_ref().expect("attr value") else {
      panic!("expected expression container");
    };

    match &container.expr {
      JSXExpr::Expr(expr) => match &**expr {
        Expr::Member(member) => {
          let Expr::Ident(object) = &*member.obj else {
            panic!("expected identifier object");
          };
          assert_eq!(object.sym.as_ref(), "styles");
        }
        other => panic!("expected member expression, found {other:?}"),
      },
      other => panic!("expected expression, found {other:?}"),
    }

    assert!(metadata.state().uses_xcss);
  }

  #[test]
  fn skips_when_no_styles_found() {
    let code = "<Component xcss={styles.primary} />";
    let (mut expr, cm, _fm) = parse_jsx_expression(code, None);
    let metadata = create_metadata(cm, Vec::new());

    let mut build_css = |_value: &Expr, _meta: &Metadata| CssOutput::new();

    let transformed = visit_xcss_prop_with_builder(&mut expr, &metadata, &mut build_css);
    assert!(!transformed);

    let Expr::JSXElement(element) = expr else {
      panic!("expected JSX element");
    };

    match element.opening.name {
      JSXElementName::Ident(ident) => assert_eq!(ident.sym.as_ref(), "Component"),
      other => panic!("expected Component element, found {other:?}"),
    }

    assert!(!metadata.state().uses_xcss);
  }

  #[test]
  fn inline_object_without_class_names_sets_undefined() {
    let code = "<Component xcss={{}} />";
    let (mut expr, cm, _fm) = parse_jsx_expression(code, None);
    let metadata = create_metadata(cm, Vec::new());

    let mut build_css = |_value: &Expr, _meta: &Metadata| CssOutput::new();

    let transformed = visit_xcss_prop_with_builder(&mut expr, &metadata, &mut build_css);
    assert!(transformed);

    let inner = find_inner_component(&expr);
    let attr = inner
            .opening
            .attrs
            .iter()
            .find_map(|attr| match attr {
                JSXAttrOrSpread::JSXAttr(attr)
                    if matches!(&attr.name, JSXAttrName::Ident(ident) if ident.sym.as_ref() == "xcss") =>
                {
                    Some(attr)
                }
                _ => None,
            })
            .expect("xcss attribute");

    let JSXAttrValue::JSXExprContainer(container) = attr.value.as_ref().expect("attr value") else {
      panic!("expected expression container");
    };

    match &container.expr {
      JSXExpr::Expr(expr) => match &**expr {
        Expr::Ident(ident) => assert_eq!(ident.sym.as_ref(), "undefined"),
        other => panic!("expected undefined identifier, found {other:?}"),
      },
      other => panic!("expected expression, found {other:?}"),
    }
  }
}
