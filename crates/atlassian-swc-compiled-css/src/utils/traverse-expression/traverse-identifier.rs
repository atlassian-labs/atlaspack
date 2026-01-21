use swc_core::ecma::ast::{Expr, Ident};

use crate::types::Metadata;
use crate::utils_create_result_pair::{ResultPair, create_result_pair};
use crate::utils_resolve_binding::resolve_binding;
use crate::utils_types::EvaluateExpression;

/// Resolves identifiers to their bound values when the binding is constant,
/// mirroring the behaviour of the Babel helper by delegating evaluation to the
/// shared `evaluate_expression` pipeline.
pub fn traverse_identifier(
  expression: &Ident,
  meta: Metadata,
  evaluate_expression: EvaluateExpression,
) -> ResultPair {
  if let Some(binding) = resolve_binding(expression.sym.as_ref(), meta.clone(), evaluate_expression)
  {
    if binding.constant {
      if let Some(node) = binding.node.as_ref() {
        let result = (evaluate_expression)(node, binding.meta.clone());
        return create_result_pair(result.value, result.meta);
      }
    }
  }

  create_result_pair(Expr::Ident(expression.clone()), meta)
}

#[cfg(test)]
mod tests {
  use super::traverse_identifier;
  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};
  use crate::utils_create_result_pair::{ResultPair, create_result_pair};
  use crate::utils_types::{BindingPath, BindingSource, PartialBindingWithMeta};
  use std::cell::RefCell;
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{DUMMY_SP, SourceMap, SyntaxContext};
  use swc_core::ecma::ast::{Expr, Ident, Lit, Str};

  fn create_metadata() -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::new(cm, Vec::new());
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

  fn identity_evaluate(expr: &Expr, meta: Metadata) -> ResultPair {
    create_result_pair(expr.clone(), meta)
  }

  #[test]
  fn returns_resolved_binding_value() {
    let meta = create_metadata();
    let binding_meta = meta.clone();
    let binding = PartialBindingWithMeta::new(
      Some(string_literal("blue")),
      Some(BindingPath::new(Some(DUMMY_SP))),
      true,
      binding_meta,
      BindingSource::Module,
    );
    meta.insert_parent_binding("color", binding);

    let ident = Ident::new("color".into(), DUMMY_SP, SyntaxContext::empty());
    let pair = traverse_identifier(&ident, meta, identity_evaluate);

    match pair.value {
      Expr::Lit(Lit::Str(Str { value, .. })) => assert_eq!(value.as_ref(), "blue"),
      other => panic!("expected string literal, found {:?}", other),
    }
  }

  #[test]
  fn returns_identifier_when_binding_missing() {
    let meta = create_metadata();
    let ident = Ident::new("color".into(), DUMMY_SP, SyntaxContext::empty());
    let pair = traverse_identifier(&ident, meta, identity_evaluate);

    match pair.value {
      Expr::Ident(returned) => assert_eq!(returned.sym.as_ref(), "color"),
      other => panic!("expected identifier, found {:?}", other),
    }
  }

  #[test]
  fn preserves_metadata_from_recursive_evaluation() {
    let meta = create_metadata();
    let inner_meta = meta.clone();
    let binding = PartialBindingWithMeta::new(
      Some(Expr::Ident(Ident::new(
        "inner".into(),
        DUMMY_SP,
        SyntaxContext::empty(),
      ))),
      None,
      true,
      inner_meta.clone(),
      BindingSource::Module,
    );
    meta.insert_parent_binding("outer", binding);

    let ident = Ident::new("outer".into(), DUMMY_SP, SyntaxContext::empty());
    let pair = traverse_identifier(&ident, meta, identity_evaluate);

    match pair.value {
      Expr::Ident(returned) => assert_eq!(returned.sym.as_ref(), "inner"),
      other => panic!("expected identifier, found {:?}", other),
    }

    assert_eq!(
      pair.meta.state().file().filename,
      inner_meta.state().file().filename
    );
  }
}
