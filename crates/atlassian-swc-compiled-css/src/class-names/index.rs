use std::collections::HashSet;

use swc_core::atoms::Atom;
use swc_core::common::{DUMMY_SP, SyntaxContext};
use swc_core::ecma::ast::{
  ArrayLit, CallExpr, Callee, Expr, ExprOrSpread, FnExpr, Ident, JSXElement, JSXElementChild,
  JSXElementName, JSXExpr, MemberProp, ObjectLit, ObjectPat, ObjectPatProp, Pat, PropName,
};
use swc_core::ecma::visit::{VisitMut, VisitMutWith};

use crate::types::Metadata;
use crate::utils_ast::{build_code_frame_error, pick_function_body};
use crate::utils_build_compiled_component::compiled_template;
use crate::utils_build_css_variables::build_css_variables;
use crate::utils_css_builders::build_css as build_css_from_expr;
use crate::utils_get_runtime_class_name_library::get_runtime_class_name_library;
use crate::utils_transform_css_items::transform_css_items;
use crate::utils_types::{CssOutput, Variable};

#[derive(Clone, Debug, PartialEq)]
pub enum ClassNamesCssNode {
  Expression(Expr),
  Expressions(Vec<Expr>),
}

fn class_names_import_contains(meta: &Metadata, name: &str) -> bool {
  meta
    .state()
    .compiled_imports
    .as_ref()
    .and_then(|imports| {
      imports
        .class_names
        .iter()
        .find(|import_name| import_name == &name)
    })
    .is_some()
}

fn build_style_expression(variables: &[Variable]) -> Expr {
  if variables.is_empty() {
    Expr::Ident(Ident::new(
      Atom::from("undefined"),
      DUMMY_SP,
      SyntaxContext::empty(),
    ))
  } else {
    Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props: build_css_variables(variables),
    })
  }
}

fn extract_property_binding_names(pattern: &Pat) -> Vec<String> {
  match pattern {
    Pat::Ident(binding) => vec![binding.id.sym.to_string()],
    Pat::Assign(assign) => extract_property_binding_names(&assign.left),
    _ => Vec::new(),
  }
}

fn collect_aliases_from_object(
  object: &ObjectPat,
  css_names: &mut HashSet<String>,
  style_names: &mut HashSet<String>,
) {
  for prop in &object.props {
    match prop {
      ObjectPatProp::KeyValue(kv) => {
        if let PropName::Ident(key) = &kv.key {
          let bindings = extract_property_binding_names(&kv.value);
          if key.sym.as_ref() == "css" {
            for binding in bindings {
              css_names.insert(binding);
            }
          } else if key.sym.as_ref() == "style" {
            for binding in bindings {
              style_names.insert(binding);
            }
          }
        }
      }
      ObjectPatProp::Assign(assign) => {
        let name = assign.key.sym.to_string();
        if name == "css" {
          css_names.insert(name);
        } else if name == "style" {
          style_names.insert(assign.key.sym.to_string());
        }
      }
      ObjectPatProp::Rest(rest) => {
        collect_aliases_from_pat(rest.arg.as_ref(), css_names, style_names);
      }
    }
  }
}

fn collect_aliases_from_fn(
  function: &FnExpr,
  css: &mut HashSet<String>,
  style: &mut HashSet<String>,
) {
  for param in &function.function.params {
    collect_aliases_from_pat(&param.pat, css, style);
  }
}

fn collect_aliases_from_params(
  params: &[Pat],
  css: &mut HashSet<String>,
  style: &mut HashSet<String>,
) {
  for pat in params {
    collect_aliases_from_pat(pat, css, style);
  }
}

fn collect_aliases_from_pat(pat: &Pat, css: &mut HashSet<String>, style: &mut HashSet<String>) {
  match pat {
    Pat::Object(object) => collect_aliases_from_object(object, css, style),
    Pat::Assign(assign) => collect_aliases_from_pat(assign.left.as_ref(), css, style),
    _ => {}
  }
}

fn collect_css_and_style_aliases(function_expr: &Expr) -> (HashSet<String>, HashSet<String>) {
  let mut css_names = HashSet::new();
  let mut style_names = HashSet::new();
  css_names.insert("css".into());
  style_names.insert("style".into());

  match function_expr {
    Expr::Arrow(arrow) => {
      collect_aliases_from_params(&arrow.params, &mut css_names, &mut style_names);
    }
    Expr::Fn(function) => {
      collect_aliases_from_fn(function, &mut css_names, &mut style_names);
    }
    _ => {}
  }

  (css_names, style_names)
}

fn runtime_call(helper: &str, class_names: Vec<Expr>) -> Expr {
  let ident = Ident::new(Atom::from(helper), DUMMY_SP, SyntaxContext::empty());
  let array = ArrayLit {
    span: DUMMY_SP,
    elems: class_names
      .into_iter()
      .map(|expr| {
        Some(ExprOrSpread {
          expr: Box::new(expr),
          spread: None,
        })
      })
      .collect(),
  };

  Expr::Call(CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Expr(Box::new(Expr::Ident(ident))),
    args: vec![ExprOrSpread {
      spread: None,
      expr: Box::new(Expr::Array(array)),
    }],
    type_args: None,
  })
}

fn extract_styles_from_expr(
  expr: &Expr,
  css_identifiers: &HashSet<String>,
) -> Option<ClassNamesCssNode> {
  match expr {
    Expr::Call(call) => {
      if call.args.is_empty() {
        return None;
      }

      match &call.callee {
        Callee::Expr(callee_expr) => match &**callee_expr {
          Expr::Ident(ident) if css_identifiers.contains(ident.sym.as_ref()) => {
            let args = call
              .args
              .iter()
              .map(|arg| (*arg.expr).clone())
              .collect::<Vec<_>>();
            Some(ClassNamesCssNode::Expressions(args))
          }
          Expr::Member(member) => {
            if matches!(
                &member.prop,
                MemberProp::Ident(prop) if prop.sym.as_ref() == "css"
            ) {
              let args = call
                .args
                .iter()
                .map(|arg| (*arg.expr).clone())
                .collect::<Vec<_>>();
              Some(ClassNamesCssNode::Expressions(args))
            } else {
              None
            }
          }
          _ => None,
        },
        _ => None,
      }
    }
    Expr::TaggedTpl(tagged) => Some(ClassNamesCssNode::Expression(Expr::Tpl(
      (*tagged.tpl).clone(),
    ))),
    _ => None,
  }
}

struct CssCallRewriter<'a, F>
where
  F: FnMut(ClassNamesCssNode, &Metadata) -> CssOutput,
{
  meta: &'a Metadata,
  build_css: &'a mut F,
  css_identifiers: &'a HashSet<String>,
  runtime_helper: &'a str,
  collected_variables: &'a mut Vec<Variable>,
  collected_sheets: &'a mut Vec<String>,
}

impl<'a, F> VisitMut for CssCallRewriter<'a, F>
where
  F: FnMut(ClassNamesCssNode, &Metadata) -> CssOutput,
{
  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    if let Some(styles) = extract_styles_from_expr(expr, self.css_identifiers) {
      let css_output = (self.build_css)(styles, self.meta);
      let transform_result = transform_css_items(&css_output.css, self.meta);

      self
        .collected_variables
        .extend(css_output.variables.into_iter());
      self
        .collected_sheets
        .extend(transform_result.sheets.into_iter());

      *expr = runtime_call(self.runtime_helper, transform_result.class_names);
      return;
    }

    expr.visit_mut_children_with(self);
  }
}

struct StylePropRewriter<'a> {
  style_identifiers: &'a HashSet<String>,
  variables: &'a [Variable],
}

impl<'a> VisitMut for StylePropRewriter<'a> {
  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    match expr {
      Expr::Ident(ident) if self.style_identifiers.contains(ident.sym.as_ref()) => {
        *expr = build_style_expression(self.variables);
        return;
      }
      Expr::Member(member) => {
        if matches!(
            &member.prop,
            MemberProp::Ident(prop) if prop.sym.as_ref() == "style"
        ) {
          *expr = build_style_expression(self.variables);
          return;
        }

        member.obj.visit_mut_with(self);
        match &mut member.prop {
          MemberProp::Computed(computed) => computed.expr.visit_mut_with(self),
          _ => {}
        }
        return;
      }
      _ => {}
    }

    expr.visit_mut_children_with(self);
  }
}

fn ensure_children_function<'a>(element: &'a mut JSXElement, meta: &Metadata) -> &'a mut Expr {
  for child in &mut element.children {
    if let JSXElementChild::JSXExprContainer(container) = child {
      if let JSXExpr::Expr(expr) = &mut container.expr {
        if matches!(expr.as_ref(), Expr::Arrow(_) | Expr::Fn(_)) {
          return expr;
        }
      }
    }
  }

  let message =
    "ClassNames children should be a function\nE.g: <ClassNames>{props => <div />}</ClassNames>";
  let error = build_code_frame_error(message, Some(element.span), meta);
  panic!("{error}");
}

/// Mirrors the Babel `visitClassNamesPath` helper by converting `<ClassNames>` JSX elements
/// into compiled components. Callers provide a CSS builder so tests can inject precomputed
/// CSS output until the native `build_css` port lands.
pub fn visit_class_names_with_builder<F>(node: &mut Expr, meta: &Metadata, mut build_css: F) -> bool
where
  F: FnMut(ClassNamesCssNode, &Metadata) -> CssOutput,
{
  let Expr::JSXElement(element) = node else {
    return false;
  };

  let JSXElementName::Ident(ident) = &element.opening.name else {
    return false;
  };

  if !class_names_import_contains(meta, ident.sym.as_ref()) {
    return false;
  }

  let runtime_helper = get_runtime_class_name_library(meta);
  let children_expr = ensure_children_function(element, meta);
  let (css_identifiers, style_identifiers) = collect_css_and_style_aliases(children_expr);

  let mut collected_variables: Vec<Variable> = Vec::new();
  let mut collected_sheets: Vec<String> = Vec::new();

  {
    let mut visitor = CssCallRewriter {
      meta,
      build_css: &mut build_css,
      css_identifiers: &css_identifiers,
      runtime_helper,
      collected_variables: &mut collected_variables,
      collected_sheets: &mut collected_sheets,
    };
    children_expr.visit_mut_with(&mut visitor);
  }

  {
    let mut visitor = StylePropRewriter {
      style_identifiers: &style_identifiers,
      variables: &collected_variables,
    };
    children_expr.visit_mut_with(&mut visitor);
  }

  let body = pick_function_body(children_expr);
  *node = compiled_template(body, &collected_sheets, meta);

  true
}

/// Convenience wrapper that delegates to the shared `build_css` helper so
/// production callers don't need to provide a custom builder.
pub fn visit_class_names(node: &mut Expr, meta: &Metadata) -> bool {
  visit_class_names_with_builder(node, meta, |css_node, metadata| match css_node {
    ClassNamesCssNode::Expression(expr) => build_css_from_expr(&expr, metadata),
    ClassNamesCssNode::Expressions(expressions) => {
      let elements = expressions
        .into_iter()
        .map(|expr| {
          Some(ExprOrSpread {
            spread: None,
            expr: Box::new(expr),
          })
        })
        .collect();

      let array = Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems: elements,
      });

      build_css_from_expr(&array, metadata)
    }
  })
}

#[cfg(test)]
mod tests {
  use super::{
    ClassNamesCssNode, class_names_import_contains, collect_css_and_style_aliases,
    visit_class_names_with_builder,
  };
  use crate::types::{CompiledImports, Metadata, PluginOptions, TransformFile, TransformState};
  use crate::utils_types::{CssItem, CssOutput};
  use std::cell::RefCell;
  use std::panic::AssertUnwindSafe;
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{FileName, SourceMap};
  use swc_core::ecma::ast::{
    Callee, Expr, JSXAttr, JSXAttrName, JSXAttrOrSpread, JSXAttrValue, JSXElementChild, JSXExpr,
    JSXExprContainer,
  };
  use swc_core::ecma::parser::lexer::Lexer;
  use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax};

  fn parse_jsx_expression(code: &str) -> Expr {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("expr.tsx".into()).into(), code.to_string());
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
    *parser.parse_expr().expect("parse expression")
  }

  fn metadata_with_class_names_import(name: &str) -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::transform_compiled_with_options(
      cm,
      Vec::new(),
      crate::types::TransformFileOptions {
        filename: Some("file.tsx".into()),
        ..Default::default()
      },
    );
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));
    {
      let mut state_mut = state.borrow_mut();
      state_mut.compiled_imports = Some(CompiledImports {
        class_names: vec![name.into()],
        ..CompiledImports::default()
      });
    }

    Metadata::new(state)
  }

  fn css_output() -> CssOutput {
    CssOutput {
      css: vec![CssItem::unconditional("._a{color:red;}")],
      variables: Vec::new(),
    }
  }

  #[test]
  fn detects_class_names_import() {
    let meta = metadata_with_class_names_import("ClassNames");
    assert!(class_names_import_contains(&meta, "ClassNames"));
    assert!(!class_names_import_contains(&meta, "Other"));
  }

  #[test]
  fn collects_aliases_from_function_params() {
    let expr = parse_jsx_expression("({ css: cx, style: styles }) => <div />");
    let (css_aliases, style_aliases) = collect_css_and_style_aliases(&expr);
    assert!(css_aliases.contains("css"));
    assert!(css_aliases.contains("cx"));
    assert!(style_aliases.contains("style"));
    assert!(style_aliases.contains("styles"));
  }

  #[test]
  fn transforms_class_names_usage() {
    let mut expr = parse_jsx_expression(
      "<ClassNames>{({ css, style }) => <div className={css({ color: 'red' })} style={style} />}</ClassNames>",
    );
    let meta = metadata_with_class_names_import("ClassNames");
    let mut received_input = Vec::new();
    let transformed = visit_class_names_with_builder(&mut expr, &meta, |node, _| {
      received_input.push(node);
      css_output()
    });

    assert!(transformed);
    assert_eq!(received_input.len(), 1);

    let Expr::JSXElement(cc_element) = &expr else {
      panic!("expected compiled component");
    };

    // The body expression is stored in the penultimate child of <CC>.
    let body_expr = match cc_element.children.get(3) {
      Some(JSXElementChild::JSXExprContainer(JSXExprContainer {
        expr: JSXExpr::Expr(inner),
        ..
      })) => inner,
      other => panic!("unexpected CC children structure: {other:?}"),
    };

    let Expr::JSXElement(div_element) = body_expr.as_ref() else {
      panic!("expected div element");
    };

    let mut class_attr = None;
    let mut style_attr = None;
    for attr in &div_element.opening.attrs {
      if let JSXAttrOrSpread::JSXAttr(JSXAttr { name, value, .. }) = attr {
        match name {
          JSXAttrName::Ident(ident) if ident.sym.as_ref() == "className" => {
            class_attr = value.clone();
          }
          JSXAttrName::Ident(ident) if ident.sym.as_ref() == "style" => {
            style_attr = value.clone();
          }
          _ => {}
        }
      }
    }

    let Some(JSXAttrValue::JSXExprContainer(class_container)) = class_attr else {
      panic!("missing className expression");
    };
    match &class_container.expr {
      JSXExpr::Expr(class_expr) => match class_expr.as_ref() {
        Expr::Call(call) => {
          let Callee::Expr(callee) = &call.callee else {
            panic!("unexpected callee");
          };
          match callee.as_ref() {
            Expr::Ident(ident) => assert!(
              ident.sym.as_ref() == "ax" || ident.sym.as_ref() == "ac",
              "unexpected runtime helper"
            ),
            other => panic!("unexpected className expression: {other:?}"),
          }
        }
        other => panic!("unexpected className expression: {other:?}"),
      },
      other => panic!("unexpected className container: {other:?}"),
    }

    let Some(JSXAttrValue::JSXExprContainer(style_container)) = style_attr else {
      panic!("missing style expression");
    };
    match &style_container.expr {
      JSXExpr::Expr(style_expr) => match style_expr.as_ref() {
        Expr::Ident(ident) => assert_eq!(ident.sym.as_ref(), "undefined"),
        other => panic!("unexpected style expression: {other:?}"),
      },
      other => panic!("unexpected style container: {other:?}"),
    }

    assert_eq!(meta.state().sheets.len(), 1);
  }

  #[test]
  fn supports_aliases_in_destructuring() {
    let mut expr = parse_jsx_expression(
      "<ClassNames>{({ css: cx, style: styles }) => <div className={cx({})} style={styles} />}</ClassNames>",
    );
    let meta = metadata_with_class_names_import("ClassNames");
    let mut called = 0;
    let transformed = visit_class_names_with_builder(&mut expr, &meta, |node, _| {
      called += 1;
      match node {
        ClassNamesCssNode::Expressions(values) => {
          assert_eq!(values.len(), 1);
        }
        other => panic!("unexpected node: {other:?}"),
      }
      css_output()
    });

    assert!(transformed);
    assert_eq!(called, 1);
  }

  #[test]
  fn handles_member_expression_calls() {
    let mut expr = parse_jsx_expression(
      "<ClassNames>{(props) => <div className={props.css({})} style={props.style} />}</ClassNames>",
    );
    let meta = metadata_with_class_names_import("ClassNames");
    let transformed = visit_class_names_with_builder(&mut expr, &meta, |_, _| css_output());
    assert!(transformed);
  }

  #[test]
  fn skips_non_class_names_elements() {
    let mut expr = parse_jsx_expression("<Other>{() => <div />}</Other>");
    let meta = metadata_with_class_names_import("ClassNames");
    let mut called = false;
    let transformed = visit_class_names_with_builder(&mut expr, &meta, |_, _| {
      called = true;
      css_output()
    });

    assert!(!transformed);
    assert!(!called);
  }

  #[test]
  fn panics_when_children_not_function() {
    let mut expr = parse_jsx_expression("<ClassNames><div /></ClassNames>");
    let meta = metadata_with_class_names_import("ClassNames");
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
      visit_class_names_with_builder(&mut expr, &meta, |_, _| css_output());
    }));

    assert!(result.is_err());
  }

  #[test]
  fn runtime_helper_switches_with_compression_map() {
    let mut expr =
      parse_jsx_expression("<ClassNames>{({ css }) => <div className={css({})} />}</ClassNames>");
    let mut options = PluginOptions::default();
    options.class_name_compression_map = Some(std::collections::BTreeMap::from([(
      "ax".into(),
      "a".into(),
    )]));
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::transform_compiled_with_options(
      cm,
      Vec::new(),
      crate::types::TransformFileOptions {
        filename: Some("file.tsx".into()),
        ..Default::default()
      },
    );
    let state = Rc::new(RefCell::new(TransformState::new(file, options)));
    {
      let mut state_mut = state.borrow_mut();
      state_mut.compiled_imports = Some(CompiledImports {
        class_names: vec!["ClassNames".into()],
        ..CompiledImports::default()
      });
    }
    let meta = Metadata::new(Rc::clone(&state));

    let transformed = visit_class_names_with_builder(&mut expr, &meta, |_, _| css_output());
    assert!(transformed);
  }
}
