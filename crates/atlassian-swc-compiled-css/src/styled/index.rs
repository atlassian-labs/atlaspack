use swc_core::common::{Span, DUMMY_SP};
use swc_core::ecma::ast::{
  ArrayLit, BinaryOp, BlockStmtOrExpr, CallExpr, Callee, Expr, ExprOrSpread, MemberExpr,
  MemberProp, Stmt, TaggedTpl,
};

use crate::types::{Metadata, Tag, TagType};
use crate::utils_ast::build_code_frame_error;
use crate::utils_build_display_name::build_display_name;
use crate::utils_build_styled_component::build_styled_component;
use crate::utils_css_builders::build_css as build_css_from_expr;
use crate::utils_types::CssOutput;

#[derive(Clone, Debug, PartialEq)]
pub enum StyledCssNode {
  Expression(Expr),
  Expressions(Vec<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
struct StyledData {
  tag: Tag,
  css_node: StyledCssNode,
}

fn styled_imports_contains(meta: &Metadata, name: &str) -> bool {
  {
    let state = meta.state();
    state
      .compiled_imports
      .as_ref()
      .map(|imports| imports.styled.iter().any(|value| value == name))
      .unwrap_or(false)
  }
}

fn extract_styled_data_from_template_literal(
  node: &TaggedTpl,
  meta: &Metadata,
) -> Option<StyledData> {
  match &*node.tag {
    Expr::Member(member) => extract_from_member_template(member, node, meta),
    Expr::Call(call) => extract_from_call_template(call, node, meta),
    _ => None,
  }
}

fn extract_from_member_template(
  member: &MemberExpr,
  node: &TaggedTpl,
  meta: &Metadata,
) -> Option<StyledData> {
  let Expr::Ident(object_ident) = &*member.obj else {
    return None;
  };

  if !styled_imports_contains(meta, object_ident.sym.as_ref()) {
    return None;
  }

  let MemberProp::Ident(prop_ident) = &member.prop else {
    return None;
  };

  let tag = Tag {
    name: prop_ident.sym.to_string(),
    tag_type: TagType::InBuiltComponent,
  };

  Some(StyledData {
    tag,
    css_node: StyledCssNode::Expression(Expr::Tpl(*node.tpl.clone())),
  })
}

fn extract_from_call_template(
  call: &CallExpr,
  node: &TaggedTpl,
  meta: &Metadata,
) -> Option<StyledData> {
  let Callee::Expr(callee_expr) = &call.callee else {
    return None;
  };

  let Expr::Ident(callee_ident) = &**callee_expr else {
    return None;
  };

  if !styled_imports_contains(meta, callee_ident.sym.as_ref()) {
    return None;
  }

  let first_arg = call.args.get(0)?;
  if first_arg.spread.is_some() {
    return None;
  }

  let Expr::Ident(component_ident) = &*first_arg.expr else {
    return None;
  };

  let tag = Tag {
    name: component_ident.sym.to_string(),
    tag_type: TagType::UserDefinedComponent,
  };

  Some(StyledData {
    tag,
    css_node: StyledCssNode::Expression(Expr::Tpl(*node.tpl.clone())),
  })
}

fn extract_styled_data_from_object_literal(node: &CallExpr, meta: &Metadata) -> Option<StyledData> {
  match &node.callee {
    Callee::Expr(expr) => match &**expr {
      Expr::Member(member) => extract_from_member_call(member, node, meta),
      Expr::Call(call) => extract_from_call_call(call, node, meta),
      _ => None,
    },
    _ => None,
  }
}

fn extract_from_member_call(
  member: &MemberExpr,
  node: &CallExpr,
  meta: &Metadata,
) -> Option<StyledData> {
  let Expr::Ident(object_ident) = &*member.obj else {
    return None;
  };

  if !styled_imports_contains(meta, object_ident.sym.as_ref()) {
    return None;
  }

  let MemberProp::Ident(prop_ident) = &member.prop else {
    return None;
  };

  let css_node = clone_arguments(node)?;

  Some(StyledData {
    tag: Tag {
      name: prop_ident.sym.to_string(),
      tag_type: TagType::InBuiltComponent,
    },
    css_node,
  })
}

fn extract_from_call_call(call: &CallExpr, node: &CallExpr, meta: &Metadata) -> Option<StyledData> {
  let Callee::Expr(callee_expr) = &call.callee else {
    return None;
  };

  let Expr::Ident(callee_ident) = &**callee_expr else {
    return None;
  };

  if !styled_imports_contains(meta, callee_ident.sym.as_ref()) {
    return None;
  }

  let first_arg = call.args.get(0)?;
  if first_arg.spread.is_some() {
    return None;
  }

  let Expr::Ident(component_ident) = &*first_arg.expr else {
    return None;
  };

  let css_node = clone_arguments(node)?;

  Some(StyledData {
    tag: Tag {
      name: component_ident.sym.to_string(),
      tag_type: TagType::UserDefinedComponent,
    },
    css_node,
  })
}

fn clone_arguments(call: &CallExpr) -> Option<StyledCssNode> {
  if call.args.is_empty() {
    return None;
  }

  let mut expressions = Vec::with_capacity(call.args.len());
  for arg in &call.args {
    if arg.spread.is_some() {
      return None;
    }
    expressions.push(*arg.expr.clone());
  }

  Some(StyledCssNode::Expressions(expressions))
}

fn extract_styled_data(node: &Expr, meta: &Metadata) -> Option<StyledData> {
  match node {
    Expr::TaggedTpl(tagged) => extract_styled_data_from_template_literal(tagged, meta),
    Expr::Call(call) => extract_styled_data_from_object_literal(call, meta),
    _ => None,
  }
}

fn has_invalid_expression(node: &TaggedTpl) -> bool {
  let has_and_logical = node.tpl.exprs.iter().any(|expr| {
    if let Expr::Arrow(arrow) = expr.as_ref() {
      if let BlockStmtOrExpr::Expr(body) = arrow.body.as_ref() {
        if let Expr::Bin(bin) = body.as_ref() {
          return matches!(bin.op, BinaryOp::LogicalAnd);
        }
      }
    }

    false
  });

  if !has_and_logical {
    return false;
  }

  for elem in &node.tpl.quasis {
    let value = elem.raw.as_ref();
    for declaration in value.split(';') {
      if let Some(index) = declaration.find(':') {
        if declaration[index + 1..].trim().is_empty() {
          return true;
        }
      }
    }
  }

  false
}

fn panic_invalid_expression(span: Span, meta: &Metadata) -> ! {
  let message = "A logical expression contains an invalid CSS declaration.\n      Compiled doesn't support CSS properties that are defined with a conditional rule that doesn't specify a default value.\n      Eg. font-weight: ${(props) => (props.isPrimary && props.isMaybe) && 'bold'}; is invalid.\n      Use ${(props) => props.isPrimary && props.isMaybe && ({ 'font-weight': 'bold' })}; instead";
  let error = build_code_frame_error(message, Some(span), meta);
  panic!("{error}");
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct StyledVisitResult {
  pub display_name: Option<Stmt>,
  pub transformed: bool,
}

/// Shared helper mirroring Babel's `visitStyledPath` implementation. The caller supplies a CSS
/// builder to ease testing until the native `build_css` port lands. When a styled usage is
/// transformed the expression is replaced in-place and an optional display name statement is
/// returned for callers that need to append it to surrounding declarations.
pub fn visit_styled_with_builder<F>(
  node: &mut Expr,
  meta: &Metadata,
  mut build_css: F,
  variable_name: Option<&str>,
) -> StyledVisitResult
where
  F: FnMut(StyledCssNode, &Metadata) -> CssOutput,
{
  let mut result = StyledVisitResult::default();

  let Some(styled_data) = extract_styled_data(node, meta) else {
    return result;
  };

  if let Expr::TaggedTpl(tagged) = node {
    if has_invalid_expression(tagged) {
      panic_invalid_expression(tagged.span, meta);
    }
  }

  let css_output = build_css(styled_data.css_node.clone(), meta);

  if std::env::var("STACK_DEBUG").is_ok() {
    eprintln!(
      "[styled] css_output sizes css={} vars={}",
      css_output.css.len(),
      css_output.variables.len()
    );
  }

  *node = build_styled_component(styled_data.tag, css_output, meta);

  if let Some(name) = variable_name {
    result.display_name = Some(build_display_name(name, None));
  }

  result.transformed = true;
  result
}

/// Public entry point that mirrors the Babel visitor by wiring in the shared
/// `build_css` implementation.
pub fn visit_styled(
  node: &mut Expr,
  meta: &Metadata,
  variable_name: Option<&str>,
) -> StyledVisitResult {
  visit_styled_with_builder(
    node,
    meta,
    |css_node, metadata| match css_node {
      StyledCssNode::Expression(expr) => build_css_from_expr(&expr, metadata),
      StyledCssNode::Expressions(expressions) => {
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
    },
    variable_name,
  )
}

trait LogicalOpExt {
  fn is_logical(&self) -> bool;
}

impl LogicalOpExt for swc_core::ecma::ast::BinaryOp {
  fn is_logical(&self) -> bool {
    matches!(
      self,
      swc_core::ecma::ast::BinaryOp::LogicalAnd
        | swc_core::ecma::ast::BinaryOp::LogicalOr
        | swc_core::ecma::ast::BinaryOp::NullishCoalescing
    )
  }
}

#[cfg(test)]
mod tests {
  use super::{
    extract_styled_data, has_invalid_expression, visit_styled_with_builder, StyledCssNode,
  };
  use crate::types::{
    CompiledImports, Metadata, PluginOptions, TagType, TransformFile, TransformState,
  };
  use crate::utils_types::{CssItem, CssOutput};
  use std::cell::RefCell;
  use std::panic::AssertUnwindSafe;
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{FileName, SourceMap};
  use swc_core::ecma::ast::{Expr, Stmt};
  use swc_core::ecma::parser::lexer::Lexer;
  use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax};

  fn parse_expression(code: &str) -> Expr {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("expr.tsx".into()).into(), code.into());
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

  fn metadata_with_styled_import(name: &str) -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::with_options(
      cm,
      Vec::new(),
      crate::types::TransformFileOptions {
        filename: Some("file.tsx".into()),
        ..Default::default()
      },
    );
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions {
        add_component_name: Some(true),
        ..PluginOptions::default()
      },
    )));
    let meta = Metadata::new(Rc::clone(&state));
    {
      let mut state_mut = state.borrow_mut();
      state_mut.compiled_imports = Some(CompiledImports {
        styled: vec![name.into()],
        ..CompiledImports::default()
      });
    }
    meta
  }

  fn css_output() -> CssOutput {
    CssOutput {
      css: vec![CssItem::unconditional("._abc123{color:red;}")],
      variables: Vec::new(),
    }
  }

  #[test]
  fn detects_invalid_expressions() {
    let expr = parse_expression(
      "styled.div`font-weight: ${(props) => (props.isPrimary && props.isMaybe) && 'bold'};`",
    );
    let Expr::TaggedTpl(tagged) = expr else {
      panic!("expected tagged template");
    };
    assert!(has_invalid_expression(&tagged));
  }

  #[test]
  fn transforms_member_expression_template_literal() {
    let mut expr = parse_expression("styled.div`color: red;`");
    let meta = metadata_with_styled_import("styled");
    let mut received_input = None;
    let result = visit_styled_with_builder(
      &mut expr,
      &meta,
      |node, _| {
        received_input = Some(node.clone());
        css_output()
      },
      Some("Component"),
    );

    assert!(matches!(received_input, Some(StyledCssNode::Expression(_))));
    assert!(result.transformed);
    assert!(matches!(expr, Expr::Call(_)));
    match result.display_name {
      Some(Stmt::If(_)) => {}
      other => panic!("unexpected display name statement: {other:?}"),
    }
  }

  #[test]
  fn transforms_call_expression_object_literal() {
    let mut expr = parse_expression("styled.div({ color: 'red' })");
    let meta = metadata_with_styled_import("styled");
    let mut received_input = None;
    let result = visit_styled_with_builder(
      &mut expr,
      &meta,
      |node, _| {
        received_input = Some(node.clone());
        css_output()
      },
      None,
    );

    assert!(result.transformed);
    assert!(
      matches!(received_input, Some(StyledCssNode::Expressions(values)) if !values.is_empty())
    );
  }

  #[test]
  fn skips_non_styled_usage() {
    let mut expr = parse_expression("notStyled.div`color: red;`");
    let meta = metadata_with_styled_import("styled");
    let mut called = false;
    let result = visit_styled_with_builder(
      &mut expr,
      &meta,
      |_, _| {
        called = true;
        css_output()
      },
      None,
    );

    assert!(!result.transformed);
    assert!(!called);
  }

  #[test]
  fn panics_for_invalid_expression() {
    let mut expr = parse_expression(
      "styled.div`font-weight: ${(props) => (props.isPrimary && props.isMaybe) && 'bold'};`",
    );
    let meta = metadata_with_styled_import("styled");
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
      visit_styled_with_builder(&mut expr, &meta, |_, _| css_output(), None);
    }));

    assert!(result.is_err());
  }

  #[test]
  fn extract_styled_data_identifies_user_defined_component() {
    let expr = parse_expression("styled(Button)`color: red;`");
    let meta = metadata_with_styled_import("styled");
    let Some(data) = extract_styled_data(&expr, &meta) else {
      panic!("expected styled data");
    };

    assert_eq!(data.tag.tag_type, TagType::UserDefinedComponent);
  }
}
