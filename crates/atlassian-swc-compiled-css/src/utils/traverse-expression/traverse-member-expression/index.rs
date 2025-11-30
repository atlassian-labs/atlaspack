use swc_core::common::SyntaxContext;
use swc_core::ecma::ast::{Expr, ExprOrSpread, Ident, MemberExpr, MemberProp};

use crate::types::Metadata;
use crate::utils_create_result_pair::{create_result_pair, ResultPair};
use crate::utils_traverse_expression_traverse_member_expression_traverse_access_path::traverse_member_access_path;
use crate::utils_types::EvaluateExpression;

struct MemberExpressionMeta {
  access_path: Vec<Ident>,
  binding_identifier: Option<Ident>,
}

fn callee_ident_from_call(call: &swc_core::ecma::ast::CallExpr) -> Option<Ident> {
  use swc_core::ecma::ast::{Callee, Expr};

  match &call.callee {
    Callee::Expr(callee) => match &**callee {
      Expr::Ident(ident) => Some(ident.clone()),
      Expr::Member(member) => match &member.prop {
        MemberProp::Ident(ident) => Some(Ident::new(
          ident.sym.clone(),
          ident.span,
          SyntaxContext::empty(),
        )),
        _ => None,
      },
      _ => None,
    },
    _ => None,
  }
}

fn collect_member_expression_meta(expression: &MemberExpr, meta: &mut MemberExpressionMeta) {
  use swc_core::ecma::ast::{Expr, MemberProp};

  // Track the property chain so we can reverse into the access path order expected
  // by the Babel helper.
  match &expression.prop {
    MemberProp::Ident(ident) => meta.access_path.push(Ident::new(
      ident.sym.clone(),
      ident.span,
      SyntaxContext::empty(),
    )),
    MemberProp::Computed(computed) => match &*computed.expr {
      Expr::Ident(ident) => meta.access_path.push(Ident::new(
        ident.sym.clone(),
        ident.span,
        SyntaxContext::empty(),
      )),
      Expr::Call(call) => {
        if let Some(ident) = callee_ident_from_call(call) {
          meta.access_path.push(ident);
        }
      }
      _ => {}
    },
    MemberProp::PrivateName(_) => {}
  }

  match expression.obj.as_ref() {
    Expr::Ident(ident) => {
      if meta.binding_identifier.is_none() {
        meta.binding_identifier = Some(ident.clone());
      }
    }
    Expr::Call(call) => {
      if let swc_core::ecma::ast::Callee::Expr(callee_expr) = &call.callee {
        match callee_expr.as_ref() {
          Expr::Ident(ident) => {
            if meta.binding_identifier.is_none() {
              meta.binding_identifier = Some(Ident::new(
                ident.sym.clone(),
                ident.span,
                SyntaxContext::empty(),
              ));
            }
          }
          Expr::Member(inner) => collect_member_expression_meta(inner, meta),
          _ => {}
        }
      }
    }
    Expr::Member(inner) => collect_member_expression_meta(inner, meta),
    _ => {}
  }
}

fn get_member_expression_meta(expression: &MemberExpr) -> MemberExpressionMeta {
  let mut meta = MemberExpressionMeta {
    access_path: Vec::new(),
    binding_identifier: None,
  };

  collect_member_expression_meta(expression, &mut meta);
  meta.access_path.reverse();

  meta
}

pub fn traverse_member_expression(
  expression: &MemberExpr,
  meta: Metadata,
  evaluate_expression: EvaluateExpression,
) -> ResultPair {
  traverse_member_expression_with_arguments(expression, meta, None, evaluate_expression)
}

pub(crate) fn traverse_member_expression_with_arguments(
  expression: &MemberExpr,
  meta: Metadata,
  call_arguments: Option<&[ExprOrSpread]>,
  evaluate_expression: EvaluateExpression,
) -> ResultPair {
  let MemberExpressionMeta {
    access_path,
    binding_identifier,
  } = get_member_expression_meta(expression);

  if let Some(binding_identifier) = binding_identifier {
    return traverse_member_access_path(
      &Expr::Ident(binding_identifier.clone()),
      meta,
      binding_identifier.sym.as_ref(),
      &access_path,
      expression,
      call_arguments,
      evaluate_expression,
    );
  }

  create_result_pair(Expr::Member(expression.clone()), meta)
}

#[cfg(test)]
mod tests {
  use super::traverse_member_expression;
  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};
  use crate::utils_create_result_pair::{create_result_pair, ResultPair};
  use crate::utils_evaluate_expression::evaluate_expression as real_evaluate_expression;
  use crate::utils_traverse_expression_traverse_call_expression::traverse_call_expression;
  use crate::utils_traverse_expression_traverse_function::traverse_function;
  use crate::utils_traverse_expression_traverse_identifier::traverse_identifier;
  use crate::utils_types::{
    BindingPath, BindingSource, EvaluateExpression, PartialBindingWithMeta,
  };
  use std::cell::RefCell;
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::Spanned;
  use swc_core::common::{FileName, SourceMap, DUMMY_SP};
  use swc_core::ecma::ast::{Callee, Expr, Lit, Number, Str};
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

  fn create_metadata() -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::new(cm, Vec::new());
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));

    Metadata::new(state)
  }

  fn expressions_are_identical(a: &Expr, b: &Expr) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b) && a.span() == b.span()
  }

  fn evaluate(expr: &Expr, meta: Metadata) -> ResultPair {
    let pair = match expr {
      Expr::Member(member) => {
        traverse_member_expression(member, meta, evaluate as EvaluateExpression)
      }
      Expr::Ident(ident) => traverse_identifier(ident, meta, evaluate as EvaluateExpression),
      Expr::Call(call) => traverse_call_expression(call, meta, evaluate as EvaluateExpression),
      Expr::Fn(_) | Expr::Arrow(_) => traverse_function(expr, meta, evaluate as EvaluateExpression),
      Expr::Paren(paren) => evaluate(&paren.expr, meta),
      _ => create_result_pair(expr.clone(), meta),
    };

    if matches!(
      pair.value,
      Expr::Lit(_) | Expr::Object(_) | Expr::TaggedTpl(_)
    ) {
      return pair;
    }

    if let Expr::Paren(inner) = &pair.value {
      return evaluate(&inner.expr, pair.meta.clone());
    }

    if !expressions_are_identical(&pair.value, expr) {
      return evaluate(&pair.value, pair.meta.clone());
    }

    create_result_pair(expr.clone(), pair.meta)
  }

  fn assert_string_literal(expr: &Expr, expected: &str) {
    match expr {
      Expr::Lit(Lit::Str(Str { value, .. })) => assert_eq!(value.as_ref(), expected),
      other => panic!("expected string literal, found {:?}", other),
    }
  }

  fn assert_number_literal(expr: &Expr, expected: f64) {
    match expr {
      Expr::Lit(Lit::Num(Number { value, .. })) => assert_eq!(*value, expected),
      other => panic!("expected number literal, found {:?}", other),
    }
  }

  #[test]
  fn resolves_simple_property_access() {
    let expr = parse_expression("colors.primary");
    let meta = create_metadata();

    let binding_expr = parse_expression("({ primary: 'blue' })");
    let binding = PartialBindingWithMeta::new(
      Some(binding_expr),
      Some(BindingPath::new(Some(DUMMY_SP))),
      true,
      meta.clone(),
      BindingSource::Module,
    );
    meta.insert_parent_binding("colors", binding);

    let Expr::Member(member) = expr else {
      panic!("expected member expression");
    };

    let pair = traverse_member_expression(&member, meta, evaluate as EvaluateExpression);
    let reduced = evaluate(&pair.value, pair.meta.clone());
    assert_string_literal(&reduced.value, "blue");
  }

  #[test]
  fn resolves_nested_member_path() {
    let expr = parse_expression("theme.colors.primary");
    let meta = create_metadata();

    let binding_expr = parse_expression("({ colors: { primary: 'navy' } })");
    let binding = PartialBindingWithMeta::new(
      Some(binding_expr),
      Some(BindingPath::new(Some(DUMMY_SP))),
      true,
      meta.clone(),
      BindingSource::Module,
    );
    meta.insert_parent_binding("theme", binding);

    let Expr::Member(member) = expr else {
      panic!("expected member expression");
    };

    let pair = traverse_member_expression(&member, meta, evaluate as EvaluateExpression);
    let reduced = evaluate(&pair.value, pair.meta.clone());
    assert_string_literal(&reduced.value, "navy");
  }

  #[test]
  fn resolves_member_function_calls() {
    let expr = parse_expression("theme.getTheme().primary");
    let meta = create_metadata();

    let binding_expr = parse_expression("({ getTheme: () => ({ primary: 'green' }) })");
    let binding = PartialBindingWithMeta::new(
      Some(binding_expr),
      Some(BindingPath::new(Some(DUMMY_SP))),
      true,
      meta.clone(),
      BindingSource::Module,
    );
    meta.insert_parent_binding("theme", binding);

    let Expr::Member(member) = expr else {
      panic!("expected member expression");
    };

    let pair = traverse_member_expression(&member, meta, evaluate as EvaluateExpression);
    let reduced = evaluate(&pair.value, pair.meta.clone());
    assert_string_literal(&reduced.value, "green");
  }

  #[test]
  fn preserves_unresolved_member_expression() {
    let expr = parse_expression("unknown.value");
    let meta = create_metadata();

    let Expr::Member(member) = expr else {
      panic!("expected member expression");
    };

    let pair = traverse_member_expression(&member, meta.clone(), evaluate as EvaluateExpression);
    match pair.value {
      Expr::Ident(ident) => assert_eq!(ident.sym.as_ref(), "unknown"),
      other => panic!("expected identifier fallback, found {:?}", other),
    }
  }

  #[test]
  fn resolves_member_expression_returning_number() {
    let expr = parse_expression("metrics.get().count");
    let meta = create_metadata();

    let binding_expr = parse_expression("({ get: () => ({ count: 42 }) })");
    let binding = PartialBindingWithMeta::new(
      Some(binding_expr),
      Some(BindingPath::new(Some(DUMMY_SP))),
      true,
      meta.clone(),
      BindingSource::Module,
    );
    meta.insert_parent_binding("metrics", binding);

    let Expr::Member(member) = expr else {
      panic!("expected member expression");
    };

    let pair = traverse_member_expression(&member, meta, evaluate as EvaluateExpression);
    let reduced = evaluate(&pair.value, pair.meta.clone());
    assert_number_literal(&reduced.value, 42.0);
  }

  #[test]
  fn resolves_computed_member_with_real_evaluator() {
    let expr = parse_expression("variantStyles[variant]");
    let meta = create_metadata();

    let binding_expr = parse_expression("css({ primary: { color: 'blue' } })");
    let binding = PartialBindingWithMeta::new(
      Some(binding_expr),
      Some(BindingPath::new(Some(DUMMY_SP))),
      true,
      meta.clone(),
      BindingSource::Module,
    );
    meta.insert_parent_binding("variantStyles", binding);

    let Expr::Member(member) = expr else {
      panic!("expected member expression");
    };

    let pair = traverse_member_expression(
      &member,
      meta,
      real_evaluate_expression as EvaluateExpression,
    );
    match pair.value {
      Expr::Call(call) => match call.callee {
        Callee::Expr(callee) => match callee.as_ref() {
          Expr::Ident(ident) => assert_eq!(ident.sym.as_ref(), "css"),
          other => panic!("expected identifier callee, found {:?}", other),
        },
        _ => panic!("expected callee expression"),
      },
      other => panic!("expected call expression, found {:?}", other),
    }
  }

  #[test]
  fn collects_binding_for_computed_member_expression() {
    let expr = parse_expression("variantStyles[variant]");

    let Expr::Member(member) = expr else {
      panic!("expected member expression");
    };

    let meta = super::get_member_expression_meta(&member);
    assert!(meta.binding_identifier.is_some());
    assert_eq!(meta.access_path.len(), 1);
    assert_eq!(
      meta.access_path[0].sym.as_ref(),
      "variant",
      "expected computed identifier to be captured in access path"
    );
  }
}
