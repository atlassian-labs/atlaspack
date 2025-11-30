use swc_core::common::Spanned;
use swc_core::common::{Span, DUMMY_SP};
use swc_core::ecma::ast::{
  ArrowExpr, BinExpr, BinaryOp, BlockStmtOrExpr, CondExpr, Expr, Lit, Str, Tpl, TplElement,
};
use swc_core::ecma::atoms::Atom;
use swc_core::ecma::visit::{noop_visit_type, Visit, VisitWith};

use crate::types::Metadata;
use crate::utils_is_empty::is_empty_value;

fn make_string_literal(value: String, span: Span) -> Expr {
  Expr::Lit(Lit::Str(Str {
    span,
    value: value.into(),
    raw: None,
  }))
}

fn normalize_raw(value: &str) -> Atom {
  Atom::from(value)
}

fn set_element_value(element: &mut TplElement, value: &str) {
  element.raw = normalize_raw(value);
  element.cooked = Some(normalize_raw(value));
}

fn append_to_start(element: &mut TplElement, prefix: &str) {
  if prefix.is_empty() {
    return;
  }

  let original_raw = element.raw.as_ref().to_string();
  let cooked_source = element
    .cooked
    .as_ref()
    .map(|value| value.as_ref().to_string())
    .unwrap_or_else(|| original_raw.clone());

  let raw = format!("{}{}", prefix, original_raw);
  element.raw = normalize_raw(&raw);

  let cooked_value = format!("{}{}", prefix, cooked_source);
  element.cooked = Some(normalize_raw(&cooked_value));
}

fn append_to_end(element: &mut TplElement, suffix: &str) {
  if suffix.is_empty() {
    return;
  }

  let original_raw = element.raw.as_ref().to_string();
  let cooked_source = element
    .cooked
    .as_ref()
    .map(|value| value.as_ref().to_string())
    .unwrap_or_else(|| original_raw.clone());

  let raw = format!("{}{}", original_raw, suffix);
  element.raw = normalize_raw(&raw);

  let cooked_value = format!("{}{}", cooked_source, suffix);
  element.cooked = Some(normalize_raw(&cooked_value));
}

pub fn recompose_template_literal(template: &mut Tpl, prefix: &str, suffix: &str) {
  if template.quasis.is_empty() {
    return;
  }

  if template.quasis.len() == 1 {
    let element = &mut template.quasis[0];
    append_to_start(element, prefix);
    append_to_end(element, suffix);
    return;
  }

  let (lead, rest) = template.quasis.split_first_mut().unwrap();
  let trail = rest.last_mut().unwrap();

  append_to_start(lead, prefix);
  append_to_end(trail, suffix);
}

fn make_template_literal(prefix: &str, suffix: &str, expression: Expr) -> Expr {
  Expr::Tpl(Tpl {
    span: DUMMY_SP,
    exprs: vec![Box::new(expression)],
    quasis: vec![
      TplElement {
        span: DUMMY_SP,
        tail: false,
        cooked: Some(normalize_raw(prefix)),
        raw: normalize_raw(prefix),
      },
      TplElement {
        span: DUMMY_SP,
        tail: true,
        cooked: Some(normalize_raw(suffix)),
        raw: normalize_raw(suffix),
      },
    ],
  })
}

fn optimize_conditional_branch(prefix: &str, suffix: &str, branch: &Expr) -> Expr {
  match branch {
    Expr::Lit(Lit::Str(str_lit)) => make_string_literal(
      format!("{}{}{}", prefix, str_lit.value, suffix),
      branch.span(),
    ),
    Expr::Lit(Lit::Num(num_lit)) => make_string_literal(
      format!("{}{}{}", prefix, num_lit.value, suffix),
      branch.span(),
    ),
    Expr::Tpl(template) => {
      let mut cloned = template.clone();
      recompose_template_literal(&mut cloned, prefix, suffix);
      Expr::Tpl(cloned)
    }
    Expr::Cond(cond) => Expr::Cond(optimize_conditional_expression(prefix, suffix, cond)),
    _ => {
      let expr = if is_empty_value(branch) {
        make_string_literal(String::new(), branch.span())
      } else {
        branch.clone()
      };

      make_template_literal(prefix, suffix, expr)
    }
  }
}

fn optimize_conditional_expression(prefix: &str, suffix: &str, expression: &CondExpr) -> CondExpr {
  let optimized_cons = optimize_conditional_branch(prefix, suffix, &expression.cons);
  let optimized_alt = optimize_conditional_branch(prefix, suffix, &expression.alt);

  CondExpr {
    span: expression.span,
    test: expression.test.clone(),
    cons: Box::new(optimized_cons),
    alt: Box::new(optimized_alt),
  }
}

fn strip_block_comments(value: &str) -> String {
  let mut result = String::with_capacity(value.len());
  let mut chars = value.chars().peekable();

  while let Some(ch) = chars.next() {
    if ch == '/' && matches!(chars.peek(), Some('*')) {
      chars.next();

      while let Some(inner) = chars.next() {
        if inner == '*' && matches!(chars.peek(), Some('/')) {
          chars.next();
          break;
        }
      }

      continue;
    }

    result.push(ch);
  }

  result
}

pub fn is_quasi_mid_statement(quasi: &TplElement) -> bool {
  let raw = quasi.raw.as_ref();
  let stripped = strip_block_comments(raw);
  let trimmed = stripped.trim_end();

  !trimmed.is_empty()
    && !trimmed.ends_with(';')
    && !trimmed.ends_with('{')
    && !trimmed.ends_with('}')
}

fn quasi_prefix(quasi: &TplElement) -> String {
  let value = quasi.raw.as_ref();
  match value.split(|c| matches!(c, ';' | '|' | '{' | '}')).last() {
    Some(segment) => segment.to_string(),
    None => String::new(),
  }
}

pub fn optimize_conditional_statement(
  quasi: &mut TplElement,
  next_quasi: Option<&mut TplElement>,
  expression: &mut ArrowExpr,
) {
  let prefix = quasi_prefix(quasi);

  if prefix.is_empty() {
    return;
  }

  let Some(next_quasi) = next_quasi else {
    return;
  };

  let next_value = next_quasi.raw.as_ref().to_string();
  let end_index = next_value.find(';');

  if end_index.is_none() {
    return;
  }

  let BlockStmtOrExpr::Expr(body_expr) = expression.body.as_mut() else {
    return;
  };

  let Expr::Cond(cond_expr) = body_expr.as_mut() else {
    return;
  };

  let suffix = &next_value[..end_index.unwrap()];
  let optimized = optimize_conditional_expression(prefix.as_str(), suffix, cond_expr);

  if optimized != cond_expr.clone() {
    *cond_expr = optimized;

    let prefix_position = quasi
      .raw
      .as_ref()
      .rfind(&prefix)
      .unwrap_or_else(|| quasi.raw.as_ref().len());
    let trimmed = quasi.raw.as_ref()[..prefix_position].to_string();
    set_element_value(quasi, &trimmed);

    let suffix_trimmed = next_value[end_index.unwrap() + 1..].to_string();
    set_element_value(next_quasi, &suffix_trimmed);
  }
}

fn is_logical_expression(expr: &Expr) -> bool {
  match expr {
    Expr::Bin(BinExpr { op, .. }) => matches!(
      op,
      BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing
    ),
    _ => false,
  }
}

fn matches_template_arrow(expr: &Expr) -> bool {
  match expr {
    Expr::Tpl(template) => template
      .exprs
      .iter()
      .any(|expression| matches!(**expression, Expr::Arrow(_))),
    _ => false,
  }
}

struct NestedConditionalVisitor<'a> {
  target: &'a Tpl,
  found: bool,
}

impl<'a> Visit for NestedConditionalVisitor<'a> {
  noop_visit_type!();

  fn visit_cond_expr(&mut self, cond: &CondExpr) {
    if self.found {
      return;
    }

    let branches = [&*cond.cons, &*cond.alt];

    for branch in branches {
      if let Expr::TaggedTpl(tagged) = branch {
        if tagged.tpl.as_ref() == self.target {
          self.found = true;
          return;
        }
      }

      if matches_template_arrow(branch) || is_logical_expression(branch) {
        self.found = true;
        return;
      }
    }

    cond.cons.visit_with(self);

    if !self.found {
      cond.alt.visit_with(self);
    }
  }
}

pub fn has_nested_template_literals_with_conditional_rules(node: &Tpl, meta: &Metadata) -> bool {
  let Some(parent) = meta.parent_expr() else {
    return false;
  };

  let mut visitor = NestedConditionalVisitor {
    target: node,
    found: false,
  };

  parent.visit_with(&mut visitor);

  visitor.found
}

#[cfg(test)]
mod tests {
  use super::{
    has_nested_template_literals_with_conditional_rules, is_quasi_mid_statement,
    optimize_conditional_statement, recompose_template_literal,
  };
  use crate::types::{
    Metadata, PluginOptions, TransformFile, TransformFileOptions, TransformState,
  };
  use std::cell::RefCell;
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{SourceMap, SyntaxContext, DUMMY_SP};
  use swc_core::ecma::ast::{
    ArrowExpr, BinExpr, BinaryOp, BlockStmtOrExpr, CondExpr, Expr, Lit, Number, Str, Tpl,
    TplElement,
  };
  use swc_core::ecma::atoms::Atom;

  fn metadata() -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::with_options(
      cm,
      Vec::new(),
      TransformFileOptions {
        filename: Some("file.tsx".into()),
        loc_filename: Some("file.tsx".into()),
        ..TransformFileOptions::default()
      },
    );
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));
    Metadata::new(state)
  }

  fn tpl_element(raw: &str, tail: bool) -> TplElement {
    TplElement {
      span: DUMMY_SP,
      tail,
      cooked: Some(Atom::from(raw)),
      raw: Atom::from(raw),
    }
  }

  #[test]
  fn detects_mid_statement_quasi() {
    let quasi = tpl_element("color:", false);
    assert!(is_quasi_mid_statement(&quasi));
  }

  #[test]
  fn detects_non_mid_statement_quasi() {
    let quasi = tpl_element("color: red;", false);
    assert!(!is_quasi_mid_statement(&quasi));
  }

  #[test]
  fn recomposes_template_literal() {
    let mut tpl = Tpl {
      span: DUMMY_SP,
      exprs: Vec::new(),
      quasis: vec![tpl_element("a", false), tpl_element("b", true)],
    };

    recompose_template_literal(&mut tpl, "x", "y");

    assert_eq!(tpl.quasis[0].raw.as_ref(), "xa");
    assert_eq!(tpl.quasis[1].raw.as_ref(), "by");
  }

  #[test]
  fn optimizes_conditional_statement() {
    let mut quasi = tpl_element("color:", false);
    let mut next = tpl_element("red;", true);
    let mut arrow = ArrowExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      params: Vec::new(),
      is_async: false,
      is_generator: false,
      type_params: None,
      return_type: None,
      body: Box::new(BlockStmtOrExpr::Expr(Box::new(Expr::Cond(CondExpr {
        span: DUMMY_SP,
        test: Box::new(Expr::Lit(Lit::Num(Number {
          span: DUMMY_SP,
          value: 1.0,
          raw: None,
        }))),
        cons: Box::new(Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          value: "green".into(),
          raw: None,
        }))),
        alt: Box::new(Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          value: "blue".into(),
          raw: None,
        }))),
      })))),
    };

    optimize_conditional_statement(&mut quasi, Some(&mut next), &mut arrow);

    if let BlockStmtOrExpr::Expr(body_expr) = arrow.body.as_ref() {
      if let Expr::Cond(cond) = body_expr.as_ref() {
        if let Expr::Lit(Lit::Str(str_lit)) = cond.cons.as_ref() {
          assert_eq!(str_lit.value.as_ref(), "color:greenred");
        } else {
          panic!("expected string literal in consequent");
        }
      } else {
        panic!("expected conditional expression");
      }
    } else {
      panic!("expected expression body");
    }

    assert_eq!(quasi.raw.as_ref(), "");
    assert_eq!(next.raw.as_ref(), "");
  }

  #[test]
  fn detects_nested_template_literals() {
    let meta = metadata();

    let nested = ArrowExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      params: Vec::new(),
      is_async: false,
      is_generator: false,
      type_params: None,
      return_type: None,
      body: Box::new(BlockStmtOrExpr::Expr(Box::new(Expr::Cond(CondExpr {
        span: DUMMY_SP,
        test: Box::new(Expr::Lit(Lit::Num(Number {
          span: DUMMY_SP,
          value: 1.0,
          raw: None,
        }))),
        cons: Box::new(Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          value: "value".into(),
          raw: None,
        }))),
        alt: Box::new(Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          value: "other".into(),
          raw: None,
        }))),
      })))),
    };

    let node = Tpl {
      span: DUMMY_SP,
      exprs: vec![Box::new(Expr::Arrow(nested))],
      quasis: vec![tpl_element("", false), tpl_element("", true)],
    };

    let parent = Expr::Cond(CondExpr {
      span: DUMMY_SP,
      test: Box::new(Expr::Lit(Lit::Num(Number {
        span: DUMMY_SP,
        value: 1.0,
        raw: None,
      }))),
      cons: Box::new(Expr::Tpl(node.clone())),
      alt: Box::new(Expr::Bin(BinExpr {
        span: DUMMY_SP,
        op: BinaryOp::LogicalAnd,
        left: Box::new(Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          value: "a".into(),
          raw: None,
        }))),
        right: Box::new(Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          value: "b".into(),
          raw: None,
        }))),
      })),
    });

    let meta = meta.with_parent_expr(Some(&parent));

    assert!(has_nested_template_literals_with_conditional_rules(
      &node, &meta
    ));
  }
}
