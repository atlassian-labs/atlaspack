use indexmap::IndexSet;
use swc_core::common::{DUMMY_SP, SyntaxContext};
use swc_core::ecma::ast::{
  CallExpr, Callee, Expr, ExprOrSpread, Ident, KeyValueProp, Lit, ParenExpr, Prop, PropName,
  PropOrSpread, Str,
};

use crate::utils_types::Variable;

fn build_arguments<F>(variable: &Variable, transform: &F) -> Vec<ExprOrSpread>
where
  F: Fn(&Expr) -> Expr,
{
  let mut args = Vec::new();
  args.push(ExprOrSpread {
    spread: None,
    expr: Box::new(transform(&variable.expression)),
  });

  if let Some(suffix) = &variable.suffix {
    args.push(ExprOrSpread {
      spread: None,
      expr: Box::new(Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: suffix.clone().into(),
        raw: None,
      }))),
    });

    if let Some(prefix) = &variable.prefix {
      args.push(ExprOrSpread {
        spread: None,
        expr: Box::new(Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          value: prefix.clone().into(),
          raw: None,
        }))),
      });
    }
  }

  args
}

fn call_expression(args: Vec<ExprOrSpread>) -> Expr {
  Expr::Call(CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
      "ix".into(),
      DUMMY_SP,
      SyntaxContext::empty(),
    )))),
    args,
    type_args: None,
  })
}

fn to_property(call: Expr, name: &str) -> PropOrSpread {
  let key = PropName::Str(Str {
    span: DUMMY_SP,
    value: name.into(),
    raw: None,
  });

  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key,
    value: Box::new(call),
  })))
}

/// Build the CSS variables object literal used for inline styles, mirroring the
/// Babel helper by deduplicating variables by name and invoking the provided
/// transform on each variable expression.
pub fn build_css_variables_with_transform<F>(
  variables: &[Variable],
  transform: F,
) -> Vec<PropOrSpread>
where
  F: Fn(&Expr) -> Expr,
{
  let mut seen = IndexSet::new();
  let mut properties = Vec::new();

  for variable in variables {
    if !seen.insert(variable.name.clone()) {
      continue;
    }

    // Apply the provided transform, then ensure callable expressions are wrapped
    // so that any immediate invocation uses Babel's `(() => ...)()` shape.
    let wrapped = |expr: &Expr| wrap_callable(&transform(expr));
    let args = build_arguments(variable, &wrapped);
    let call = call_expression(args);
    properties.push(to_property(call, &variable.name));
  }

  properties
}

/// Convenience wrapper that mirrors the Babel helper's default transform by
/// leaving expressions untouched other than wrapping callable callees to match
/// Babel's IIFE formatting for inline variable evaluation.
pub fn build_css_variables(variables: &[Variable]) -> Vec<PropOrSpread> {
  build_css_variables_with_transform(variables, |expr| wrap_callable(expr))
}

fn wrap_callable(expr: &Expr) -> Expr {
  match expr {
    // Parenthesize function/arrow callees so IIFEs format like Babel's `(() => ...)()`.
    Expr::Call(call) => {
      if let Callee::Expr(callee_expr) = &call.callee {
        match callee_expr.as_ref() {
          Expr::Arrow(_) | Expr::Fn(_) => {
            let mut new_call = call.clone();
            new_call.callee = Callee::Expr(Box::new(Expr::Paren(ParenExpr {
              span: DUMMY_SP,
              expr: callee_expr.clone(),
            })));
            return Expr::Call(new_call);
          }
          _ => {}
        }
      }
      expr.clone()
    }
    Expr::Arrow(_) | Expr::Fn(_) => Expr::Paren(ParenExpr {
      span: DUMMY_SP,
      expr: Box::new(expr.clone()),
    }),
    _ => expr.clone(),
  }
}

#[cfg(test)]
mod tests {
  use super::{build_css_variables, build_css_variables_with_transform};
  use crate::utils_types::Variable;
  use swc_core::common::{DUMMY_SP, SyntaxContext};
  use swc_core::ecma::ast::{Expr, Ident, Lit, Prop, PropName, PropOrSpread, Str};

  fn ident(name: &str) -> Expr {
    Expr::Ident(Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty()))
  }

  fn variable(name: &str, expression: Expr) -> Variable {
    Variable {
      name: name.into(),
      expression,
      prefix: None,
      suffix: None,
    }
  }

  #[test]
  fn deduplicates_variables_and_builds_properties() {
    let vars = vec![
      variable("--color", ident("value")),
      variable("--color", ident("other")),
      Variable {
        name: "--size".into(),
        expression: ident("size"),
        prefix: Some("prefix".into()),
        suffix: Some("suffix".into()),
      },
    ];

    let props = build_css_variables(&vars);
    assert_eq!(props.len(), 2);

    match &props[0] {
      PropOrSpread::Prop(prop) => match &**prop {
        Prop::KeyValue(kv) => {
          match &kv.key {
            PropName::Str(value) => assert_eq!(value.value.as_ref(), "--color"),
            _ => panic!("expected string key"),
          }

          match &*kv.value {
            Expr::Call(call) => {
              assert_eq!(call.args.len(), 1);
              match &*call.args[0].expr {
                Expr::Ident(ident) => assert_eq!(ident.sym.as_ref(), "value"),
                _ => panic!("expected identifier expression"),
              }
            }
            _ => panic!("expected call expression"),
          }
        }
        _ => panic!("expected key value property"),
      },
      _ => panic!("expected property"),
    }

    match &props[1] {
      PropOrSpread::Prop(prop) => match &**prop {
        Prop::KeyValue(kv) => {
          match &kv.key {
            PropName::Str(value) => assert_eq!(value.value.as_ref(), "--size"),
            _ => panic!("expected string key"),
          }

          match &*kv.value {
            Expr::Call(call) => {
              assert_eq!(call.args.len(), 3);
              match &*call.args[1].expr {
                Expr::Lit(Lit::Str(str)) => {
                  assert_eq!(str.value.as_ref(), "suffix");
                }
                _ => panic!("expected suffix literal"),
              }
              match &*call.args[2].expr {
                Expr::Lit(Lit::Str(str)) => {
                  assert_eq!(str.value.as_ref(), "prefix");
                }
                _ => panic!("expected prefix literal"),
              }
            }
            _ => panic!("expected call expression"),
          }
        }
        _ => panic!("expected key value property"),
      },
      _ => panic!("expected property"),
    }
  }

  #[test]
  fn applies_transform_to_variable_expression() {
    let vars = vec![variable("--color", ident("value"))];

    let props = build_css_variables_with_transform(&vars, |expr| match expr {
      Expr::Ident(_) => Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: "transformed".into(),
        raw: None,
      })),
      _ => expr.clone(),
    });

    match &props[0] {
      PropOrSpread::Prop(prop) => match &**prop {
        Prop::KeyValue(kv) => match &*kv.value {
          Expr::Call(call) => {
            assert_eq!(call.args.len(), 1);
            match &*call.args[0].expr {
              Expr::Lit(Lit::Str(str)) => {
                assert_eq!(str.value.as_ref(), "transformed");
              }
              _ => panic!("expected transformed literal"),
            }
          }
          _ => panic!("expected call expression"),
        },
        _ => panic!("expected key value property"),
      },
      _ => panic!("expected property"),
    }
  }

  #[test]
  fn omits_prefix_when_suffix_missing() {
    let vars = vec![Variable {
      name: "--alpha".into(),
      expression: ident("value"),
      prefix: Some("prefix".into()),
      suffix: None,
    }];

    let props = build_css_variables(&vars);
    match &props[0] {
      PropOrSpread::Prop(prop) => match &**prop {
        Prop::KeyValue(kv) => match &*kv.value {
          Expr::Call(call) => {
            assert_eq!(call.args.len(), 1);
          }
          _ => panic!("expected call expression"),
        },
        _ => panic!("expected key value property"),
      },
      _ => panic!("expected property"),
    }
  }
}
