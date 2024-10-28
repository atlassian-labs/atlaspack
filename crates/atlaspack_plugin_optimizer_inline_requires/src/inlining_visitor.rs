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

    // Expressions are wrapped in (0, require(...))
    // The reason this is required is due to the following output being treated
    // differently:
    //
    // ```
    // const value = { default: class Something {} };
    // new value.default() // => this is instance of Something
    //
    // // however
    // const getValue = () => value;
    // new getValue().default() // => this fails because `getValue` is not a constructor
    //
    // // and
    // new (0, getValue()).default() // => this works and uses `default` as the constructor
    // ```
    *n = swc_core::quote!("(0, $expr)" as Expr, expr: Expr = replacement_expression.clone());
  }
}
