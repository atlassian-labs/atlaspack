use std::collections::HashMap;
use swc_core::ecma::ast::{Expr, Id};
use swc_core::ecma::visit::{VisitMut, VisitMutWith};

/// Given a set of variable IDs and a replacement expressions, this visitor will replace all
/// identifiers that match said ID with the replacement.
#[derive(Default)]
pub struct IdentifierReplacementVisitor {
  /// Replacement map for `Id` scope aware values. We can add another structure for symbol based
  /// replacement.
  ///
  /// We could also generalise this a bit and have it handle finding the binding before inlining.
  id_replacement: HashMap<Id, Expr>,
}

impl IdentifierReplacementVisitor {
  pub fn add_replacement(&mut self, id: Id, expr: Expr) {
    self.id_replacement.insert(id, expr);
  }
}

impl VisitMut for IdentifierReplacementVisitor {
  fn visit_mut_expr(&mut self, n: &mut Expr) {
    let Expr::Ident(ident) = n else {
      n.visit_mut_children_with(self);
      return;
    };
    let Some(replacement_expression) = self.id_replacement.get(&ident.to_id()) else {
      return;
    };
    *n = replacement_expression.clone();
  }
}
