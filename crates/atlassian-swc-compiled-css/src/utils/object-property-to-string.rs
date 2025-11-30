use swc_core::common::Spanned;
use swc_core::ecma::ast::{
  BinExpr, BinaryOp, CallExpr, Callee, ComputedPropName, Expr, KeyValueProp, Lit, MemberProp,
  Number, PropName, Str, Tpl,
};

use crate::types::Metadata;
use crate::utils_evaluate_expression::evaluate_expression;

/// Converts a template literal into a string by evaluating each embedded expression and
/// concatenating it with the surrounding quasi values. Mirrors the behaviour of the Babel helper
/// by reusing the metadata returned from each evaluation when recursing.
fn template_literal_to_string(template: &Tpl, meta: Metadata) -> String {
  let mut result = String::new();

  for (index, quasi) in template.quasis.iter().enumerate() {
    result.push_str(quasi.raw.as_ref());

    if let Some(expression) = template.exprs.get(index) {
      let pair = evaluate_expression(expression, meta.clone());
      result.push_str(&expression_to_string(&pair.value, pair.meta));
    }
  }

  result
}

/// Converts a binary expression to a string by statically concatenating operands when possible.
fn binary_expression_to_string(expression: &BinExpr, meta: Metadata) -> String {
  if expression.op != BinaryOp::Add {
    panic!(
      "Cannot use {} for string operation. Use + for string concatenation",
      operator_to_string(expression.op)
    );
  }

  let left = expression_to_string(&expression.left, meta.clone());
  let right = expression_to_string(&expression.right, meta);

  format!("{}{}", left, right)
}

/// Describes a call expression that represents `<string>.concat(...)`.
fn is_string_concat_expression(expression: &Expr) -> Option<&CallExpr> {
  let call = match expression {
    Expr::Call(call) => call,
    _ => return None,
  };

  let callee = match &call.callee {
    Callee::Expr(expr) => expr,
    _ => return None,
  };

  let member = match &**callee {
    Expr::Member(member) => member,
    _ => return None,
  };

  let property = match &member.prop {
    MemberProp::Ident(ident) => ident,
    _ => return None,
  };

  if property.sym.as_ref() != "concat" {
    return None;
  }

  if matches!(&*member.obj, Expr::Lit(Lit::Str(_))) {
    Some(call)
  } else {
    None
  }
}

/// Determines whether a string concat expression can be statically concatenated by inspecting
/// argument types.
pub(crate) fn can_be_statically_concatenated(call: &CallExpr) -> bool {
  let expr = Expr::Call(call.clone());
  if is_string_concat_expression(&expr).is_none() {
    return false;
  }

  call.args.iter().all(|arg| {
    if arg.spread.is_some() {
      return false;
    }

    matches!(
      &*arg.expr,
      Expr::Lit(Lit::Str(_)) | Expr::Lit(Lit::Num(_)) | Expr::Tpl(_)
    )
  })
}

/// Performs the static concatenation for a string `.concat()` call.
fn concat_to_string(call: &CallExpr, meta: Metadata) -> String {
  let callee = match &call.callee {
    Callee::Expr(expr) => expr,
    _ => panic!("Cannot concatenate an expression with non-expression arguments"),
  };

  let base = match &**callee {
    Expr::Member(member) => match &*member.obj {
      Expr::Lit(Lit::Str(lit)) => lit.value.to_string(),
      _ => panic!("Cannot concatenate an expression with non-expression arguments"),
    },
    _ => panic!("Cannot concatenate base expression"),
  };

  call.args.iter().fold(base, |mut acc, arg| {
    if arg.spread.is_some() {
      panic!("Cannot concatenate an expression with non-expression arguments");
    }

    acc.push_str(&expression_to_string(&arg.expr, meta.clone()));
    acc
  })
}

fn literal_to_string(lit: &Lit) -> String {
  match lit {
    Lit::Str(Str { value, .. }) => value.to_string(),
    Lit::Num(Number { value, .. }) => value.to_string(),
    Lit::BigInt(bigint) => bigint.value.to_string(),
    _ => panic!("{} has no name.'", literal_type(lit)),
  }
}

fn literal_type(lit: &Lit) -> &'static str {
  match lit {
    Lit::Str(_) => "StringLiteral",
    Lit::Num(_) => "NumericLiteral",
    Lit::Bool(_) => "BooleanLiteral",
    Lit::Null(_) => "NullLiteral",
    Lit::BigInt(_) => "BigIntLiteral",
    Lit::Regex(_) => "RegExpLiteral",
    Lit::JSXText(_) => "JSXText",
  }
}

fn operator_to_string(op: BinaryOp) -> &'static str {
  match op {
    BinaryOp::EqEq => "==",
    BinaryOp::NotEq => "!=",
    BinaryOp::EqEqEq => "===",
    BinaryOp::NotEqEq => "!==",
    BinaryOp::Lt => "<",
    BinaryOp::LtEq => "<=",
    BinaryOp::Gt => ">",
    BinaryOp::GtEq => ">=",
    BinaryOp::LShift => "<<",
    BinaryOp::RShift => ">>",
    BinaryOp::ZeroFillRShift => ">>>",
    BinaryOp::Add => "+",
    BinaryOp::Sub => "-",
    BinaryOp::Mul => "*",
    BinaryOp::Div => "/",
    BinaryOp::Mod => "%",
    BinaryOp::BitOr => "|",
    BinaryOp::BitXor => "^",
    BinaryOp::BitAnd => "&",
    BinaryOp::LogicalOr => "||",
    BinaryOp::LogicalAnd => "&&",
    BinaryOp::In => "in",
    BinaryOp::InstanceOf => "instanceof",
    BinaryOp::Exp => "**",
    BinaryOp::NullishCoalescing => "??",
  }
}

pub(crate) fn expression_type(expr: &Expr) -> &'static str {
  match expr {
    Expr::Array(_) => "ArrayExpression",
    Expr::Arrow(_) => "ArrowFunctionExpression",
    Expr::Assign(_) => "AssignmentExpression",
    Expr::Await(_) => "AwaitExpression",
    Expr::Bin(_) => "BinaryExpression",
    Expr::Call(_) => "CallExpression",
    Expr::Class(_) => "ClassExpression",
    Expr::Cond(_) => "ConditionalExpression",
    Expr::Fn(_) => "FunctionExpression",
    Expr::Ident(_) => "Identifier",
    Expr::Lit(lit) => literal_type(lit),
    Expr::Member(_) => "MemberExpression",
    Expr::New(_) => "NewExpression",
    Expr::Object(_) => "ObjectExpression",
    Expr::Paren(_) => "ParenthesizedExpression",
    Expr::Tpl(_) => "TemplateLiteral",
    Expr::This(_) => "ThisExpression",
    Expr::Unary(_) => "UnaryExpression",
    Expr::Update(_) => "UpdateExpression",
    Expr::Yield(_) => "YieldExpression",
    Expr::MetaProp(_) => "MetaProperty",
    Expr::SuperProp(_) => "Super",
    Expr::OptChain(_) => "OptionalChain",
    Expr::TsAs(_) => "TsAsExpression",
    Expr::TsConstAssertion(_) => "TsConstAssertion",
    Expr::TsInstantiation(_) => "TsInstantiationExpression",
    Expr::TsNonNull(_) => "TsNonNullExpression",
    Expr::TsTypeAssertion(_) => "TsTypeAssertion",
    Expr::Invalid(_) => "InvalidExpression",
    Expr::JSXElement(_) => "JSXElement",
    Expr::JSXFragment(_) => "JSXFragment",
    Expr::TaggedTpl(_) => "TaggedTemplateExpression",
    Expr::Seq(_) => "SequenceExpression",
    _ => "Expression",
  }
}

pub(crate) fn expression_to_string(expression: &Expr, meta: Metadata) -> String {
  match expression {
    Expr::Lit(lit) => literal_to_string(lit),
    Expr::Ident(_) | Expr::Member(_) => {
      let pair = evaluate_expression(expression, meta.clone());
      if pair.value == *expression {
        // If we couldn't resolve the identifier/member, leave the key empty to match
        // Babel's non-panicking behaviour when objectPropertyToString can't evaluate.
        return String::new();
      }

      expression_to_string(&pair.value, pair.meta)
    }
    Expr::Tpl(template) => template_literal_to_string(template, meta),
    Expr::Bin(bin) => binary_expression_to_string(bin, meta),
    Expr::Call(_) => {
      if let Some(concat_call) = is_string_concat_expression(expression) {
        if can_be_statically_concatenated(concat_call) {
          return concat_to_string(concat_call, meta);
        }
      }

      let kind = expression_type(expression);
      if matches!(kind, "Identifier" | "MemberExpression") {
        return String::new();
      }

      panic!("Cannot statically evaluate the value of \"{}\"", kind);
    }
    Expr::TsConstAssertion(assertion) => expression_to_string(&assertion.expr, meta),
    Expr::TsAs(assertion) => expression_to_string(&assertion.expr, meta),
    Expr::TsTypeAssertion(assertion) => expression_to_string(&assertion.expr, meta),
    Expr::TsNonNull(assertion) => expression_to_string(&assertion.expr, meta),
    Expr::Paren(paren) => expression_to_string(&paren.expr, meta),
    _ => panic!("{} has no name.'", expression_type(expression)),
  }
}

/// Returns the key name for an object property, mirroring the Babel helper.
pub fn object_property_to_string(prop: &KeyValueProp, meta: Metadata) -> String {
  match &prop.key {
    PropName::Ident(ident) => ident.sym.to_string(),
    PropName::Str(str_lit) => str_lit.value.to_string(),
    PropName::Num(num_lit) => num_lit.value.to_string(),
    PropName::BigInt(big) => big.value.to_string(),
    PropName::Computed(ComputedPropName { expr, .. }) => {
      if std::env::var("STACK_DEBUG_PROP").is_ok() {
        eprintln!(
          "[object_property_to_string] computed key expr_type={} span={:?}",
          expression_type(expr),
          expr.span()
        );
      }
      expression_to_string(expr, meta)
    }
  }
}

/// Exposed for other helpers that need to convert expressions into string keys.
pub fn expression_value_to_string(expression: &Expr, meta: Metadata) -> String {
  expression_to_string(expression, meta)
}

#[cfg(test)]
mod tests {
  use super::{expression_value_to_string, object_property_to_string};
  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};
  use crate::utils_types::{BindingPath, BindingSource, PartialBindingWithMeta};
  use std::cell::RefCell;
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{SourceMap, SyntaxContext, DUMMY_SP};
  use swc_core::ecma::ast::{
    BinExpr, BinaryOp, CallExpr, Callee, ComputedPropName, Expr, ExprOrSpread, Ident, KeyValueProp,
    Lit, MemberExpr, MemberProp, Number, PropName, Str, Tpl, TplElement,
  };

  fn create_metadata() -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::new(cm.clone(), Vec::new());
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));

    Metadata::new(state)
  }

  fn string_literal(value: &str) -> Expr {
    Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: value.into(),
      raw: None,
    }))
  }

  fn numeric_literal(value: f64) -> Expr {
    Expr::Lit(Lit::Num(Number {
      span: DUMMY_SP,
      value,
      raw: None,
    }))
  }

  fn make_key_value_prop(name: &str, computed: bool) -> KeyValueProp {
    if computed {
      KeyValueProp {
        key: PropName::Computed(ComputedPropName {
          span: DUMMY_SP,
          expr: Box::new(Expr::Ident(Ident::new(
            name.into(),
            DUMMY_SP,
            SyntaxContext::empty(),
          ))),
        }),
        value: Box::new(string_literal("value")),
      }
    } else {
      KeyValueProp {
        key: PropName::Ident(Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty()).into()),
        value: Box::new(string_literal("value")),
      }
    }
  }

  #[test]
  fn returns_identifier_key_when_not_computed() {
    let prop = make_key_value_prop("name", false);
    let meta = create_metadata();

    assert_eq!(object_property_to_string(&prop, meta), "name");
  }

  #[test]
  fn resolves_computed_identifier_key() {
    let prop = make_key_value_prop("field", true);
    let meta = create_metadata();

    let binding = PartialBindingWithMeta::new(
      Some(string_literal("field-name")),
      Some(BindingPath::new(Some(DUMMY_SP))),
      true,
      meta.clone(),
      BindingSource::Module,
    );
    meta.insert_parent_binding("field", binding);

    assert_eq!(object_property_to_string(&prop, meta), "field-name");
  }

  #[test]
  fn empty_when_identifier_cannot_be_resolved() {
    let prop = make_key_value_prop("missing", true);
    let meta = create_metadata();

    assert_eq!(object_property_to_string(&prop, meta), "");
  }

  #[test]
  fn string_literal_keys_are_returned() {
    let prop = KeyValueProp {
      key: PropName::Str(Str {
        span: DUMMY_SP,
        value: "name".into(),
        raw: None,
      }),
      value: Box::new(string_literal("value")),
    };

    let meta = create_metadata();
    assert_eq!(object_property_to_string(&prop, meta), "name");
  }

  #[test]
  fn numeric_literal_keys_are_returned() {
    let prop = KeyValueProp {
      key: PropName::Num(Number {
        span: DUMMY_SP,
        value: 123.0,
        raw: None,
      }),
      value: Box::new(string_literal("value")),
    };

    let meta = create_metadata();
    assert_eq!(object_property_to_string(&prop, meta), "123");
  }

  #[test]
  fn template_literal_keys_are_combined() {
    let template = Tpl {
      span: DUMMY_SP,
      exprs: vec![
        Box::new(string_literal("id")),
        Box::new(string_literal("hidden")),
      ],
      quasis: vec![
        TplElement {
          span: DUMMY_SP,
          tail: false,
          cooked: None,
          raw: "".into(),
        },
        TplElement {
          span: DUMMY_SP,
          tail: false,
          cooked: None,
          raw: ", ".into(),
        },
        TplElement {
          span: DUMMY_SP,
          tail: true,
          cooked: None,
          raw: "".into(),
        },
      ],
    };

    let prop = KeyValueProp {
      key: PropName::Computed(ComputedPropName {
        span: DUMMY_SP,
        expr: Box::new(Expr::Tpl(template)),
      }),
      value: Box::new(string_literal("value")),
    };

    let meta = create_metadata();
    assert_eq!(object_property_to_string(&prop, meta), "id, hidden");
  }

  #[test]
  fn binary_expression_concatenation() {
    let expr = Expr::Bin(BinExpr {
      span: DUMMY_SP,
      op: BinaryOp::Add,
      left: Box::new(string_literal("foo")),
      right: Box::new(Expr::Bin(BinExpr {
        span: DUMMY_SP,
        op: BinaryOp::Add,
        left: Box::new(string_literal("bar")),
        right: Box::new(numeric_literal(1.0)),
      })),
    });

    let result = expression_value_to_string(&expr, create_metadata());
    assert_eq!(result, "foobar1");
  }

  #[test]
  #[should_panic(expected = "Cannot use * for string operation. Use + for string concatenation")]
  fn throws_on_illegal_string_operator() {
    let expr = Expr::Bin(BinExpr {
      span: DUMMY_SP,
      op: BinaryOp::Mul,
      left: Box::new(string_literal("foo")),
      right: Box::new(string_literal("bar")),
    });

    expression_value_to_string(&expr, create_metadata());
  }

  #[test]
  #[should_panic(expected = "ArrayExpression has no name.'")]
  fn throws_on_unsupported_expression() {
    expression_value_to_string(&Expr::Array(Default::default()), create_metadata());
  }

  #[test]
  fn concat_expression_combines_arguments() {
    let call = Expr::Call(CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          value: "base ".into(),
          raw: None,
        }))
        .into(),
        prop: MemberProp::Ident(
          Ident::new("concat".into(), DUMMY_SP, SyntaxContext::empty()).into(),
        ),
      }))),
      args: vec![
        ExprOrSpread {
          spread: None,
          expr: Box::new(string_literal("one")),
        },
        ExprOrSpread {
          spread: None,
          expr: Box::new(numeric_literal(2.0)),
        },
      ],
      type_args: None,
    });

    let result = expression_value_to_string(&call, create_metadata());
    assert_eq!(result, "base one2");
  }
}
