use std::ops::Index;

use indexmap::IndexMap;
use swc_core::common::Span;
use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::{
  BlockStmt, Expr, Id, Ident, ModuleItem, Pat, Stmt, VarDecl, VarDeclKind, VarDeclarator,
};
use swc_core::ecma::visit::{Visit, VisitMut, VisitMutWith, VisitWith};

/// Tracks usage count of identifiers in a scope
#[derive(Default)]
struct UsageCounter {
  usage_counts: IndexMap<Id, usize>,
}

impl Visit for UsageCounter {
  fn visit_expr(&mut self, n: &Expr) {
    if let Expr::Ident(ident) = n {
      *self.usage_counts.entry(ident.to_id()).or_insert(0) += 1;
    }
    n.visit_children_with(self);
  }
}

/// Information about a reuse binding
struct ReuseRequireBinding {
  /// The unique variable identifier for the reuse binding
  var_ident: Ident,
  /// The expression to bind
  expr: Expr,
  /// Whether the binding has been created yet
  created: bool,
}

/// Given a set of variable IDs and a replacement expressions, this visitor will replace all
/// identifiers that match said ID with the replacement.
///
/// For identifiers used multiple times in the same scope, it creates reuse bindings to avoid
/// calling require() multiple times unnecessarily.
#[derive(Default)]
pub struct IdentifierReplacementVisitor {
  /// Replacement map for `Id` scope aware values. We can add another structure for symbol based
  /// replacement.
  id_replacement: IndexMap<Id, Expr>,
  /// Reuse bindings for identifiers used multiple times
  reuse_require_bindings: IndexMap<Id, ReuseRequireBinding>,
}

impl IdentifierReplacementVisitor {
  pub fn add_replacement(&mut self, id: Id, expr: Expr) {
    self.id_replacement.insert(id, expr);
  }

  /// Analyze usage counts and determine which identifiers need reuse bindings
  fn analyze_scope(&mut self, node: &impl VisitWith<UsageCounter>) {
    let mut counter = UsageCounter::default();
    node.visit_with(&mut counter);

    // Clear any existing reuse bindings for this scope
    self.reuse_require_bindings.clear();

    // Create reuse bindings for identifiers used more than once
    for (id, count) in counter.usage_counts {
      if count > 1 && self.id_replacement.contains_key(&id) {
        let expr = self.id_replacement[&id].clone();

        let var_ident: Ident = Ident::new_private("__inlineRequire".into(), DUMMY_SP);
        self.reuse_require_bindings.insert(
          id,
          ReuseRequireBinding {
            var_ident,
            expr,
            created: false,
          },
        );
      }
    }
  }

  /// Create a reuse binding variable declaration
  fn create_reuse_require_binding(&mut self, id: &Id) -> Option<Stmt> {
    let reuse_binding: &mut ReuseRequireBinding = self.reuse_require_bindings.get_mut(id)?;
    if reuse_binding.created {
      return None;
    }

    reuse_binding.created = true;
    let var_ident = reuse_binding.var_ident.clone();
    let expr = reuse_binding.expr.clone();

    // Wrap in (0, expr) like in visit_mut_expr
    let wrapped_expr = swc_core::quote!("(0, $expr)" as Expr, expr: Expr = expr);

    Some(Stmt::Decl(swc_core::ecma::ast::Decl::Var(Box::new(
      VarDecl {
        kind: VarDeclKind::Const,
        decls: vec![VarDeclarator {
          span: Span::default(),
          name: Pat::Ident(swc_core::ecma::ast::BindingIdent::from(var_ident)),
          init: Some(Box::new(wrapped_expr)),
          definite: false,
        }],
        declare: false,
        span: Span::default(),
        ctxt: Default::default(),
      },
    ))))
  }
}

impl VisitMut for IdentifierReplacementVisitor {
  fn visit_mut_block_stmt(&mut self, n: &mut BlockStmt) {
    // Track usage in this scope (block level = depth 2)
    self.analyze_scope(n);

    // Process children first
    n.visit_mut_children_with(self);

    // Insert reuse binding declarations at the beginning of the block
    let mut reuse_decls = Vec::new();

    let ids: Vec<_> = self.id_replacement.keys().cloned().collect();
    for id in ids {
      if let Some(decl) = self.create_reuse_require_binding(&id) {
        reuse_decls.push(decl);
      }
    }

    // Insert reuse declarations at the beginning of the block
    if !reuse_decls.is_empty() {
      let mut new_stmts = reuse_decls;
      new_stmts.append(&mut n.stmts);
      n.stmts = new_stmts;
    }
  }

  fn visit_mut_expr(&mut self, n: &mut Expr) {
    let Expr::Ident(ident) = n else {
      n.visit_mut_children_with(self);
      return;
    };

    let id = ident.to_id();

    // Check if this identifier has a reuse binding
    if let Some(reuse_binding) = self.reuse_require_bindings.get(&id) {
      *n = Expr::Ident(reuse_binding.var_ident.clone());
      return;
    }

    // Otherwise directly insert the require expression
    let Some(replacement_expression) = self.id_replacement.get(&id) else {
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
