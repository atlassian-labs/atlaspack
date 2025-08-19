use std::collections::{HashMap, HashSet};
use swc_core::atoms::Atom;
use swc_core::ecma::ast::{Expr, Id, Ident};
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
  id_replacement_with_reuse_var_ident: HashMap<Id, IdReplacement>,
  pub reuse_var_idents: HashSet<Ident>,
  pub is_reused_inline_requires_enabled: bool,
}

struct IdReplacement {
  expr: Expr,
  reuse_var_ident: Ident,
}

impl IdentifierReplacementVisitor {
  pub fn new(is_reused_inline_requires_enabled: bool) -> Self {
    Self {
      is_reused_inline_requires_enabled,
      ..Default::default()
    }
  }

  pub fn add_replacement_with_reuse_var_ident(
    &mut self,
    id: Id,
    expr: Expr,
    reuse_var_ident: Ident,
  ) {
    self.id_replacement_with_reuse_var_ident.insert(
      id,
      IdReplacement {
        expr,
        reuse_var_ident,
      },
    );
  }

  pub fn add_replacement(&mut self, id: Id, expr: Expr) {
    self.id_replacement.insert(id, expr);
  }

  pub fn add_reuse_var_ident(&mut self, ident: Ident) {
    self.reuse_var_idents.insert(ident);
  }
}

impl VisitMut for IdentifierReplacementVisitor {
  fn visit_mut_expr(&mut self, n: &mut Expr) {
    let Expr::Ident(ident) = n else {
      n.visit_mut_children_with(self);
      return;
    };

    if self.is_reused_inline_requires_enabled {
      let Some(IdReplacement {
        expr: replacement_expression,
        reuse_var_ident,
      }) = self.id_replacement_with_reuse_var_ident.get(&ident.to_id())
      else {
        return;
      };

      *n = swc_core::quote!(
        "($var || ($var = $expr))" as Expr,
        var: Ident = reuse_var_ident.clone(),
        expr: Expr = replacement_expression.clone()
      );
    } else {
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
}
