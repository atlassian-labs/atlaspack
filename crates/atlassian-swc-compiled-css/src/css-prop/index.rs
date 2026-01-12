use swc_core::common::Span;
use swc_core::ecma::ast::{Expr, JSXAttr, JSXAttrName, JSXAttrOrSpread, JSXAttrValue, JSXExpr};

use crate::constants::{
  COMPILED_DIRECTIVE_DISABLE_LINE, COMPILED_DIRECTIVE_DISABLE_NEXT_LINE,
  COMPILED_DIRECTIVE_TRANSFORM_CSS_PROP,
};
use crate::types::Metadata;
use crate::utils_build_compiled_component::build_compiled_component;
use crate::utils_comments::get_node_comments;
use crate::utils_css_builders::build_css as build_css_from_expr;
use crate::utils_types::CssOutput;
use swc_core::ecma::ast::JSXElement;

fn is_css_attribute(attr: &JSXAttr) -> bool {
  matches!(
      &attr.name,
      JSXAttrName::Ident(ident) if ident.sym.as_ref() == "css"
  )
}

fn attribute_value_to_expression(value: &JSXAttrValue) -> Expr {
  match value {
    JSXAttrValue::Lit(lit) => match lit {
      swc_core::ecma::ast::Lit::Str(str_lit) => {
        Expr::Lit(swc_core::ecma::ast::Lit::Str(str_lit.clone()))
      }
      _ => panic!("Value of JSX attribute was unexpected."),
    },
    JSXAttrValue::JSXExprContainer(container) => match &container.expr {
      JSXExpr::Expr(expr) => *expr.clone(),
      JSXExpr::JSXEmptyExpr(_) => panic!("Value of JSX attribute was unexpected."),
    },
    _ => panic!("Value of JSX attribute was unexpected."),
  }
}

fn is_css_prop_disabled(span: Span, meta: &Metadata) -> bool {
  let disable_line_directive = format!(
    "{} {}",
    COMPILED_DIRECTIVE_DISABLE_LINE, COMPILED_DIRECTIVE_TRANSFORM_CSS_PROP
  );
  let disable_next_line_directive = format!(
    "{} {}",
    COMPILED_DIRECTIVE_DISABLE_NEXT_LINE, COMPILED_DIRECTIVE_TRANSFORM_CSS_PROP
  );
  let comments = get_node_comments(span, meta);

  let before_disabled = comments.before.iter().any(|comment| {
    comment
      .text
      .as_ref()
      .trim()
      .starts_with(&disable_next_line_directive)
  });

  let current_disabled = comments.current.iter().any(|comment| {
    comment
      .text
      .as_ref()
      .trim()
      .starts_with(&disable_line_directive)
  });

  before_disabled || current_disabled
}

fn build_css_from_attribute<F>(attr: &JSXAttr, meta: &Metadata, build_css: &F) -> CssOutput
where
  F: Fn(&Expr, &Metadata) -> CssOutput,
{
  let Some(value) = &attr.value else {
    return CssOutput::new();
  };

  let expression = attribute_value_to_expression(value);
  build_css(&expression, meta)
}

/// Mirrors the Babel `visitCssPropPath` helper by converting `css` attributes on JSX
/// elements into compiled components. The caller provides the top-level expression so
/// this function can mutate it in-place, and a CSS builder which allows tests to inject
/// precomputed CSS output until the native `build_css` port lands.
pub fn visit_css_prop_with_builder<F>(node: &mut Expr, meta: &Metadata, build_css: F)
where
  F: Fn(&Expr, &Metadata) -> CssOutput,
{
  let Expr::JSXElement(element) = node else {
    return;
  };

  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
    use swc_core::ecma::ast::JSXElementName;
    let name = match &element.opening.name {
      JSXElementName::Ident(id) => id.sym.as_ref().to_string(),
      _ => String::from("<complex>"),
    };
    let attrs: Vec<String> = element
      .opening
      .attrs
      .iter()
      .map(|a| match a {
        JSXAttrOrSpread::JSXAttr(attr) => match &attr.name {
          JSXAttrName::Ident(id) => id.sym.as_ref().to_string(),
          _ => String::from("<computed>"),
        },
        JSXAttrOrSpread::SpreadElement(_) => String::from("<spread>"),
      })
      .collect();
    eprintln!(
      "[css-prop] visit element name={} attrs=[{}] span={:?}",
      name,
      attrs.join(","),
      element.opening.span
    );
  }

  let css_prop_index =
    element
      .opening
      .attrs
      .iter()
      .enumerate()
      .find_map(|(index, attr)| match attr {
        JSXAttrOrSpread::JSXAttr(attr) if is_css_attribute(attr) => Some(index),
        _ => None,
      });

  let Some(index) = css_prop_index else {
    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      eprintln!(
        "[css-prop] no css attribute on element span={:?}",
        element.opening.span
      );
    }
    return;
  };

  let attr_clone = match &element.opening.attrs[index] {
    JSXAttrOrSpread::JSXAttr(attr) => attr.clone(),
    _ => return,
  };

  if attr_clone.value.is_none() {
    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      eprintln!(
        "[css-prop] css attribute has no value span={:?}",
        attr_clone.span
      );
    }
    return;
  }

  if is_css_prop_disabled(element.opening.span, meta) || is_css_prop_disabled(attr_clone.span, meta)
  {
    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      eprintln!(
        "[css-prop] css prop disabled via directive span_el={:?} span_attr={:?}",
        element.opening.span, attr_clone.span
      );
    }
    return;
  }

  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
    eprintln!("[css-prop] found css attribute span={:?}", attr_clone.span);
  }

  if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
    eprintln!("[visit_css_prop_with_builder] calling build_css_from_attribute");
  }
  let css_output = build_css_from_attribute(&attr_clone, meta, &build_css);
  if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
    eprintln!(
      "[visit_css_prop_with_builder] build_css_from_attribute done, css_count={}",
      css_output.css.len()
    );
  }

  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
    let state = meta.state();
    let css_alias = state.imported_compiled_imports.css.clone();
    let filename = state.filename.clone().unwrap_or_default();
    eprintln!(
      "[css-prop] file={} css_alias={:?} css_count={} variables={} span={:?} style_rules={} sheets={} uses_runtime_wrappers={}",
      filename,
      css_alias,
      css_output.css.len(),
      css_output.variables.len(),
      element.opening.span,
      state.style_rules.len(),
      state.sheets.len(),
      state.uses_runtime_wrappers
    );
  }

  element.opening.attrs.remove(index);

  if css_output.css.is_empty() {
    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      eprintln!(
        "[css-prop] built empty css output span={:?}",
        element.opening.span
      );
    }
    return;
  }

  if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
    eprintln!(
      "[visit_css_prop_with_builder] creating jsx_expr and calling build_compiled_component"
    );
  }
  let jsx_expr = Expr::JSXElement(element.as_ref().clone().into());
  let replacement = build_compiled_component(jsx_expr, &css_output, meta);
  if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
    eprintln!("[visit_css_prop_with_builder] build_compiled_component done");
  }

  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
    eprintln!(
      "[css-prop] replaced element span={:?} uses_runtime_wrappers={}",
      element.opening.span,
      meta.state().uses_runtime_wrappers
    );
  }

  if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
    eprintln!("[visit_css_prop_with_builder] assigning replacement to node");
  }
  *node = replacement;
  if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
    eprintln!("[visit_css_prop_with_builder] END");
  }
}

/// Convenience wrapper that mirrors the Babel visitor by invoking the shared
/// `build_css` helper when transforming a `css` prop.
pub fn visit_css_prop(node: &mut Expr, meta: &Metadata) {
  if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
    eprintln!("[visit_css_prop] START");
  }
  visit_css_prop_with_builder(node, meta, build_css_from_expr);
  if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
    eprintln!("[visit_css_prop] END");
  }
}

/// Transform a css prop on a JSXElement directly (for nested elements),
/// mirroring the logic of `visit_css_prop_with_builder` but working in-place.
pub fn visit_css_prop_on_element(element: &mut JSXElement, meta: &Metadata) {
  // Find css attribute
  let css_prop_index =
    element
      .opening
      .attrs
      .iter()
      .enumerate()
      .find_map(|(index, attr)| match attr {
        JSXAttrOrSpread::JSXAttr(attr) if is_css_attribute(attr) => Some(index),
        _ => None,
      });

  let Some(index) = css_prop_index else {
    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      use swc_core::ecma::ast::JSXElementName;
      let name = match &element.opening.name {
        JSXElementName::Ident(id) => id.sym.as_ref().to_string(),
        _ => String::from("<complex>"),
      };
      eprintln!(
        "[css-prop:on-element] no css attribute on element name={} span={:?}",
        name, element.opening.span
      );
    }
    return;
  };

  let attr_clone = match &element.opening.attrs[index] {
    JSXAttrOrSpread::JSXAttr(attr) => attr.clone(),
    _ => return,
  };

  if attr_clone.value.is_none() {
    return;
  }

  if is_css_prop_disabled(element.opening.span, meta) || is_css_prop_disabled(attr_clone.span, meta)
  {
    return;
  }

  // Build CSS from the attribute value
  let css_output = build_css_from_attribute(&attr_clone, meta, &build_css_from_expr);

  // Remove the css attribute regardless of whether CSS was produced
  element.opening.attrs.remove(index);

  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
    eprintln!(
      "[css-prop:on-element] built css items={} span={:?}",
      css_output.css.len(),
      element.opening.span
    );
  }

  if css_output.css.is_empty() {
    return;
  }

  // Replace current element with compiled wrapper
  let jsx_expr = Expr::JSXElement(element.clone().into());
  let replacement = build_compiled_component(jsx_expr, &css_output, meta);
  if let Expr::JSXElement(new_el) = replacement {
    *element = *new_el;
  }
}

#[cfg(test)]
mod tests {
  use super::visit_css_prop_with_builder;
  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};
  use crate::utils_types::{CssItem, CssOutput};
  use std::cell::RefCell;
  use std::rc::Rc;
  use swc_core::common::comments::{Comment, CommentKind};
  use swc_core::common::sync::Lrc;
  use swc_core::common::{FileName, SourceMap, Span};
  use swc_core::ecma::ast::{Expr, JSXAttrName, JSXAttrOrSpread, JSXElementName};
  use swc_core::ecma::parser::lexer::Lexer;
  use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax};

  fn parse_jsx_expression(code: &str) -> (Expr, Lrc<SourceMap>, Lrc<swc_core::common::SourceFile>) {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("expr.jsx".into()).into(), code.into());
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

  fn span_for(file: &swc_core::common::SourceFile, start: usize, len: usize) -> Span {
    let base = file.start_pos.0;
    let lo = swc_core::common::BytePos(base + start as u32);
    let hi = swc_core::common::BytePos(base + (start + len) as u32);
    Span::new(lo, hi)
  }

  fn line_comment(file: &swc_core::common::SourceFile, code: &str, marker: &str) -> Comment {
    let start = code.find(marker).expect("marker missing");
    let end = code[start..]
      .find('\n')
      .map(|idx| start + idx)
      .unwrap_or_else(|| code.len());
    let span = span_for(file, start, end - start);
    let text = code[start + 2..end].to_string();

    Comment {
      kind: CommentKind::Line,
      span,
      text: text.into(),
    }
  }

  #[test]
  fn replaces_element_when_css_output_present() {
    let code = "<div css={{ color: 'red' }} />";
    let (mut expr, cm, _fm) = parse_jsx_expression(code);
    let metadata = create_metadata(cm, Vec::new());

    let captured: Rc<RefCell<Option<Expr>>> = Rc::new(RefCell::new(None));
    let capture = Rc::clone(&captured);

    visit_css_prop_with_builder(&mut expr, &metadata, |value, _| {
      capture.borrow_mut().replace(value.clone());
      CssOutput {
        css: vec![CssItem::unconditional(
          "._1wyb1fwx{font-size:12px}".to_string(),
        )],
        variables: Vec::new(),
      }
    });

    assert!(captured.borrow().is_some(), "builder should be invoked");

    match expr {
      Expr::JSXElement(element) => match &element.opening.name {
        JSXElementName::Ident(ident) => {
          assert_eq!(ident.sym.as_ref(), "CC");
        }
        other => panic!("expected CC wrapper, found {:?}", other),
      },
      other => panic!("expected JSX element replacement, found {:?}", other),
    }
  }

  #[test]
  fn removes_css_attribute_when_no_css_output() {
    let code = "<div css={{}} />";
    let (mut expr, cm, _fm) = parse_jsx_expression(code);
    let metadata = create_metadata(cm, Vec::new());

    visit_css_prop_with_builder(&mut expr, &metadata, |_value, _| CssOutput::new());

    let Expr::JSXElement(element) = expr else {
      panic!("expected JSX element");
    };

    assert!(
      element.opening.attrs.is_empty(),
      "css prop should be removed"
    );
  }

  #[test]
  fn sets_runtime_wrappers_flag_when_css_output_present() {
    let code = "<div css={{ color: 'red' }} />";
    let (mut expr, cm, _fm) = parse_jsx_expression(code);
    let metadata = create_metadata(cm, Vec::new());

    {
      let state = metadata.state();
      assert!(
        !state.uses_runtime_wrappers,
        "flag should be false before transform"
      );
    }

    visit_css_prop_with_builder(&mut expr, &metadata, |_value, _| CssOutput {
      css: vec![CssItem::unconditional(
        "._1wyb1fwx{font-size:12px}".to_string(),
      )],
      variables: Vec::new(),
    });

    let state = metadata.state();
    assert!(
      state.uses_runtime_wrappers,
      "flag should be true after css prop transform"
    );
  }

  #[test]
  fn skips_transform_when_disabled_via_comment() {
    let code = "// @compiled-disable-next-line transform-css-prop\n<div css={{}} />";
    let (mut expr, cm, fm) = parse_jsx_expression(code);
    let comment = line_comment(
      &fm,
      code,
      "// @compiled-disable-next-line transform-css-prop",
    );
    let metadata = create_metadata(cm, vec![comment]);

    visit_css_prop_with_builder(&mut expr, &metadata, |_value, _| {
      panic!("builder should not be invoked when disabled");
    });

    let Expr::JSXElement(element) = expr else {
      panic!("expected JSX element");
    };

    let css_attr = element.opening.attrs.iter().any(|attr| match attr {
      JSXAttrOrSpread::JSXAttr(inner) => match &inner.name {
        JSXAttrName::Ident(ident) => ident.sym.as_ref() == "css",
        _ => false,
      },
      _ => false,
    });

    assert!(css_attr, "css attribute should remain when disabled");
  }
}
