use swc_core::ecma::ast::Expr;

use crate::types::Metadata;

/// Represents the result of evaluating an expression helper.
#[derive(Clone, Debug)]
pub struct ResultPair {
  pub value: Expr,
  pub meta: Metadata,
}

/// Mirrors the Babel `createResultPair` helper by pairing the produced value
/// with the metadata that should accompany subsequent traversals.
pub fn create_result_pair(value: Expr, meta: Metadata) -> ResultPair {
  ResultPair { value, meta }
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;
  use std::rc::Rc;

  use swc_core::common::sync::Lrc;
  use swc_core::common::SourceMap;
  use swc_core::ecma::ast::{Expr, Lit, Str};

  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};

  use super::create_result_pair;

  #[test]
  fn pairs_value_and_metadata() {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::new(cm.clone(), Vec::new());
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));
    let meta = Metadata::new(state);
    let value = Expr::Lit(Lit::Str(Str {
      span: Default::default(),
      value: "test".into(),
      raw: None,
    }));

    let pair = create_result_pair(value.clone(), meta.clone());

    assert_eq!(pair.value, value);
    assert_eq!(
      pair.meta.state().file().filename,
      meta.state().file().filename
    );
  }
}
