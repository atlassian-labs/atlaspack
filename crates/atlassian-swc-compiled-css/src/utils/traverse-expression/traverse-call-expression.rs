use swc_core::common::Spanned;
use swc_core::common::{DUMMY_SP, Span};
use swc_core::ecma::ast::{
  Callee, Expr, IdentName, MemberExpr, MemberProp, ObjectPat, ObjectPatProp, Pat, PropName,
};

use crate::types::Metadata;
use crate::utils_create_result_pair::{ResultPair, create_result_pair};
use crate::utils_resolve_binding::resolve_binding;
use crate::utils_traverse_expression_traverse_function::traverse_function;
use crate::utils_traverse_expression_traverse_member_expression::traverse_member_expression_with_arguments;
use crate::utils_traversers_object::get_object_property_value;
use crate::utils_types::{BindingPath, BindingSource, EvaluateExpression, PartialBindingWithMeta};

fn insert_binding(meta: &Metadata, name: &str, value: Expr, span: Option<Span>) {
  let binding = PartialBindingWithMeta::new(
    Some(value),
    Some(BindingPath::new(span)),
    true,
    meta.clone(),
    BindingSource::Module,
  );

  meta.insert_own_binding(name.to_string(), binding);
}

fn skip_parens<'a>(expr: &'a Expr) -> &'a Expr {
  match expr {
    Expr::Paren(inner) => skip_parens(&inner.expr),
    _ => expr,
  }
}

fn clone_without_parens(expr: &Expr) -> Expr {
  match expr {
    Expr::Paren(inner) => clone_without_parens(&inner.expr),
    _ => expr.clone(),
  }
}

fn property_name_as_string(name: &PropName) -> Option<String> {
  match name {
    PropName::Ident(ident) => Some(ident.sym.as_ref().to_string()),
    PropName::Str(str_name) => Some(str_name.value.to_string()),
    _ => None,
  }
}

fn member_expression_for_property(argument: &Expr, property: &str) -> Expr {
  Expr::Member(MemberExpr {
    span: argument.span(),
    obj: Box::new(argument.clone()),
    prop: MemberProp::Ident(IdentName::new(property.into(), DUMMY_SP)),
  })
}

fn extract_property_value(argument: &Expr, property: &str) -> Option<Expr> {
  match argument {
    Expr::Object(object) => get_object_property_value(object, property).map(|result| result.node),
    Expr::TsAs(ts_as) => extract_property_value(&ts_as.expr, property),
    Expr::Paren(paren) => extract_property_value(&paren.expr, property),
    Expr::Ident(_) | Expr::Member(_) | Expr::Call(_) | Expr::Fn(_) | Expr::Arrow(_) => {
      Some(member_expression_for_property(argument, property))
    }
    _ => None,
  }
}

fn bind_object_pattern(
  pattern: &ObjectPat,
  argument: &Expr,
  meta: &Metadata,
  evaluate_expression: EvaluateExpression,
) {
  for prop in &pattern.props {
    match prop {
      ObjectPatProp::KeyValue(key_value) => {
        if let Some(name) = property_name_as_string(&key_value.key) {
          let value = extract_property_value(argument, &name);
          bind_pattern(&key_value.value, value, meta, evaluate_expression);
        }
      }
      ObjectPatProp::Assign(assign) => {
        let name = assign.key.sym.as_ref();
        let value = extract_property_value(argument, name).or_else(|| {
          assign
            .value
            .as_ref()
            .map(|expr| (evaluate_expression)(expr, meta.clone()).value)
        });

        if let Some(value) = value {
          insert_binding(meta, name, value, Some(assign.key.span));
        }
      }
      ObjectPatProp::Rest(_) => {}
    }
  }
}

fn bind_pattern(
  pattern: &Pat,
  argument: Option<Expr>,
  meta: &Metadata,
  evaluate_expression: EvaluateExpression,
) {
  match pattern {
    Pat::Ident(binding_ident) => {
      if let Some(argument) = argument {
        insert_binding(
          meta,
          binding_ident.id.sym.as_ref(),
          argument,
          Some(binding_ident.id.span),
        );
      }
    }
    Pat::Object(object_pattern) => {
      if let Some(argument) = argument.as_ref() {
        bind_object_pattern(object_pattern, argument, meta, evaluate_expression);
      }
    }
    Pat::Assign(assign) => {
      let value = argument.or_else(|| {
        let evaluated = (evaluate_expression)(&assign.right, meta.clone());
        Some(evaluated.value)
      });
      bind_pattern(&assign.left, value, meta, evaluate_expression);
    }
    _ => {}
  }
}

fn collect_function_params(expr: &Expr) -> Vec<&Pat> {
  match expr {
    Expr::Fn(fn_expr) => fn_expr
      .function
      .params
      .iter()
      .map(|param| &param.pat)
      .collect(),
    Expr::Arrow(arrow) => arrow.params.iter().collect(),
    _ => Vec::new(),
  }
}

pub fn traverse_call_expression(
  expression: &swc_core::ecma::ast::CallExpr,
  meta: Metadata,
  evaluate_expression: EvaluateExpression,
) -> ResultPair {
  let mut updated_meta = meta;

  let Callee::Expr(callee_expr) = &expression.callee else {
    return create_result_pair(Expr::Call(expression.clone()), updated_meta);
  };

  let raw_callee_expr = callee_expr.as_ref();
  let callee_expr = skip_parens(raw_callee_expr);

  if let Expr::Member(member) = callee_expr {
    if matches!(member.prop, MemberProp::Ident(_)) {
      return traverse_member_expression_with_arguments(
        member,
        updated_meta,
        Some(&expression.args),
        evaluate_expression,
      );
    }
  }

  let mut function_expression: Option<Expr> = None;

  match callee_expr {
    Expr::Fn(_) | Expr::Arrow(_) => {
      function_expression = Some(clone_without_parens(raw_callee_expr));
    }
    Expr::Ident(ident) => {
      if let Some(binding) = resolve_binding(
        ident.sym.as_ref(),
        updated_meta.clone(),
        evaluate_expression,
      ) {
        if binding.constant {
          if let Some(node) = binding.node.as_ref() {
            if matches!(node, Expr::Fn(_) | Expr::Arrow(_)) {
              function_expression = Some(node.clone());
            }
          }
        }
      }
    }
    _ => {}
  }

  if let Some(function_expr) = function_expression {
    let mut evaluated_arguments: Vec<Expr> = Vec::new();

    for argument in &expression.args {
      let result = (evaluate_expression)(&argument.expr, updated_meta.clone());
      evaluated_arguments.push(result.value);
    }

    let params = collect_function_params(&function_expr);

    if !params.is_empty() {
      let own_scope = updated_meta.allocate_own_scope();
      updated_meta = updated_meta.with_own_scope(Some(own_scope));

      for (index, pattern) in params.iter().enumerate() {
        let argument = evaluated_arguments.get(index).cloned();
        bind_pattern(pattern, argument, &updated_meta, evaluate_expression);
      }
    }

    // After binding parameters, directly evaluate the function body with the
    // updated metadata that contains the parameter bindings. This is critical
    // because calling evaluate_expression on an identifier would resolve the
    // binding and use binding.meta instead of updated_meta, losing the
    // parameter bindings.
    return traverse_function(&function_expr, updated_meta, evaluate_expression);
  }

  (evaluate_expression)(callee_expr, updated_meta)
}

#[cfg(test)]
mod tests {
  use super::traverse_call_expression;
  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};
  use crate::utils_create_result_pair::{ResultPair, create_result_pair};
  use crate::utils_traverse_expression_traverse_function::traverse_function;
  use crate::utils_traverse_expression_traverse_identifier::traverse_identifier;
  use crate::utils_traverse_expression_traverse_member_expression::traverse_member_expression;
  use crate::utils_types::{
    BindingPath, BindingSource, EvaluateExpression, PartialBindingWithMeta,
  };
  use std::cell::RefCell;
  use std::rc::Rc;
  use swc_core::common::Spanned;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{DUMMY_SP, FileName, SourceMap};
  use swc_core::ecma::ast::{Expr, Lit, Number, Str};
  use swc_core::ecma::parser::lexer::Lexer;
  use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax};

  fn parse_expression(code: &str) -> Expr {
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
      Expr::Call(call) => traverse_call_expression(call, meta, evaluate as EvaluateExpression),
      Expr::Member(member) => {
        traverse_member_expression(member, meta, evaluate as EvaluateExpression)
      }
      Expr::Ident(ident) => traverse_identifier(ident, meta, evaluate as EvaluateExpression),
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
  fn evaluates_inline_function_call() {
    let expr = parse_expression("(() => 'blue')()");
    let meta = create_metadata();

    let Expr::Call(call) = expr else {
      panic!("expected call expression");
    };

    let pair = traverse_call_expression(&call, meta, evaluate as EvaluateExpression);
    let reduced = evaluate(&pair.value, pair.meta.clone());
    assert_string_literal(&reduced.value, "blue");
  }

  #[test]
  fn resolves_identifier_function_binding() {
    let expr = parse_expression("getColor()");
    let meta = create_metadata();

    let binding_expr = parse_expression("() => 'green'");
    let binding = PartialBindingWithMeta::new(
      Some(binding_expr),
      Some(BindingPath::new(Some(DUMMY_SP))),
      true,
      meta.clone(),
      BindingSource::Module,
    );
    meta.insert_parent_binding("getColor", binding);

    let Expr::Call(call) = expr else {
      panic!("expected call expression");
    };

    let pair = traverse_call_expression(&call, meta, evaluate as EvaluateExpression);
    let reduced = evaluate(&pair.value, pair.meta.clone());
    assert_string_literal(&reduced.value, "green");
  }

  #[test]
  fn maps_object_pattern_parameters() {
    let expr = parse_expression("(function ({ color }) { return color; })({ color: 'red' })");
    let meta = create_metadata();

    let Expr::Call(call) = expr else {
      panic!("expected call expression");
    };

    let pair = traverse_call_expression(&call, meta, evaluate as EvaluateExpression);
    let reduced = evaluate(&pair.value, pair.meta.clone());
    assert_string_literal(&reduced.value, "red");
  }

  #[test]
  fn evaluates_member_expression_callees() {
    let expr = parse_expression("theme.getColor()");
    let meta = create_metadata();

    let binding_expr = parse_expression("({ getColor: () => 'navy' })");
    let binding = PartialBindingWithMeta::new(
      Some(binding_expr),
      Some(BindingPath::new(Some(DUMMY_SP))),
      true,
      meta.clone(),
      BindingSource::Module,
    );
    meta.insert_parent_binding("theme", binding);

    let Expr::Call(call) = expr else {
      panic!("expected call expression");
    };

    let pair = traverse_call_expression(&call, meta, evaluate as EvaluateExpression);
    let reduced = evaluate(&pair.value, pair.meta.clone());
    assert_string_literal(&reduced.value, "navy");
  }

  #[test]
  fn preserves_non_function_calls() {
    let expr = parse_expression("(value => value)(10)");
    let meta = create_metadata();

    let Expr::Call(call) = expr else {
      panic!("expected call expression");
    };

    let pair = traverse_call_expression(&call, meta, evaluate as EvaluateExpression);
    let reduced = evaluate(&pair.value, pair.meta.clone());
    assert_number_literal(&reduced.value, 10.0);
  }

  #[test]
  fn evaluates_function_returning_object_with_bound_parameters() {
    // Test case mirroring the backgroundLines function pattern:
    // const fn = (a, b) => ({ prop1: a, prop2: b })
    // fn('value1', 'value2')
    let expr = parse_expression("((a, b) => ({ prop1: a, prop2: b }))('value1', 'value2')");
    let meta = create_metadata();

    let Expr::Call(call) = expr else {
      panic!("expected call expression");
    };

    let pair = traverse_call_expression(&call, meta, evaluate as EvaluateExpression);

    // The result should be an object expression
    match pair.value {
      Expr::Object(object) => {
        assert_eq!(object.props.len(), 2);
      }
      other => panic!("expected object expression, found {:?}", other),
    }
  }

  #[test]
  fn evaluates_bound_function_returning_object() {
    // Test case where the function is bound to a variable:
    // const myFunc = (x) => ({ result: x })
    // myFunc('test')
    let expr = parse_expression("myFunc('test')");
    let meta = create_metadata();

    let binding_expr = parse_expression("(x) => ({ result: x })");
    let binding = PartialBindingWithMeta::new(
      Some(binding_expr),
      Some(BindingPath::new(Some(DUMMY_SP))),
      true,
      meta.clone(),
      BindingSource::Module,
    );
    meta.insert_parent_binding("myFunc", binding);

    let Expr::Call(call) = expr else {
      panic!("expected call expression");
    };

    let pair = traverse_call_expression(&call, meta, evaluate as EvaluateExpression);

    // The result should be an object expression
    match pair.value {
      Expr::Object(object) => {
        assert_eq!(object.props.len(), 1);
      }
      other => panic!("expected object expression, found {:?}", other),
    }
  }
}
