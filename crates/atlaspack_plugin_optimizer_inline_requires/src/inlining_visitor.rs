use indexmap::IndexMap;
use swc_core::common::Span;
use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::{
  BlockStmt, Expr, Id, Ident, Pat, Stmt, VarDecl, VarDeclKind, VarDeclarator,
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
#[derive(Clone)]
struct ReuseRequireBinding {
  /// The unique variable identifier for the reuse binding  
  var_ident: Ident,
  /// The expression to bind
  expr: Expr,
}

/// Placeholder for a binding that will be created later
#[derive(Clone)]
struct PendingBinding {
  /// The expression to bind
  expr: Expr,
}

/// Context for a block scope
#[derive(Default)]
struct ScopeContext {
  /// Which identifiers have multiple usages and need reuse bindings (pending)
  needs_reuse_binding: IndexMap<Id, PendingBinding>,
  /// Which identifiers have already had their reuse binding created
  reuse_binding_created: IndexMap<Id, ReuseRequireBinding>,
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
  /// Stack of scope contexts for nested scopes
  scope_stack: Vec<ScopeContext>,
  /// Global counter for unique names across all scopes
  global_name_counter: usize,
  pub is_reused_inline_requires_enabled: bool,
}

impl IdentifierReplacementVisitor {
  pub fn new(is_reused_inline_requires_enabled: bool) -> Self {
    Self {
      is_reused_inline_requires_enabled,
      ..Default::default()
    }
  }

  pub fn add_replacement(&mut self, id: Id, expr: Expr) {
    self.id_replacement.insert(id, expr);
  }

  /// Analyze usage patterns in a scope and set up reuse bindings for identifiers used multiple times
  fn analyze_scope(&mut self, node: &impl VisitWith<UsageCounter>) {
    let mut counter = UsageCounter::default();
    node.visit_with(&mut counter);

    // Collect identifiers that need bindings
    let mut bindings_to_create = Vec::new();

    // Create pending bindings for identifiers used more than once
    for (id, count) in counter.usage_counts {
      if count > 1 && self.id_replacement.contains_key(&id) {
        let expr = self.id_replacement[&id].clone();

        let pending_binding = PendingBinding { expr };

        bindings_to_create.push((id, pending_binding));
      }
    }

    // Add bindings to current scope
    if let Some(current_scope) = self.scope_stack.last_mut() {
      for (id, binding) in bindings_to_create {
        current_scope.needs_reuse_binding.insert(id, binding);
      }
    }
  }

  /// Find a reuse binding for an identifier, looking through the scope stack
  fn find_reuse_binding(&self, id: &Id) -> Option<&ReuseRequireBinding> {
    // Look through scope stack from innermost to outermost
    for scope in self.scope_stack.iter().rev() {
      if let Some(binding) = scope.reuse_binding_created.get(id) {
        return Some(binding);
      }
    }
    None
  }

  /// Check if we need to create a reuse binding for this identifier at this point
  fn should_create_reuse_binding(&self, id: &Id) -> bool {
    if let Some(current_scope) = self.scope_stack.last() {
      // We should create a binding if:
      // 1. This identifier needs a reuse binding in the current scope
      // 2. The reuse binding hasn't been created yet in this scope
      // 3. No parent scope has already created a binding that we can inherit
      current_scope.needs_reuse_binding.contains_key(id)
        && !current_scope.reuse_binding_created.contains_key(id)
        && self.find_reuse_binding(id).is_none()
    } else {
      false
    }
  }

  /// Create and mark a reuse binding as created
  fn create_reuse_binding(&mut self, id: &Id) -> Option<ReuseRequireBinding> {
    let current_scope = self.scope_stack.last_mut()?;
    let pending_binding = current_scope.needs_reuse_binding.get(id)?.clone();

    // Generate unique variable name at creation time
    let suffix = if self.global_name_counter == 0 {
      String::new()
    } else {
      self.global_name_counter.to_string()
    };
    self.global_name_counter += 1;

    let var_name = format!("__inlineRequireReuse{}", suffix);
    let var_ident = Ident::new_private(var_name.into(), DUMMY_SP);

    let binding = ReuseRequireBinding {
      var_ident,
      expr: pending_binding.expr,
    };

    current_scope
      .reuse_binding_created
      .insert(id.clone(), binding.clone());
    Some(binding)
  }

  /// Create a variable declaration for a reuse binding
  fn create_reuse_declaration(&self, binding: &ReuseRequireBinding) -> Stmt {
    let wrapped_expr = swc_core::quote!("(0, $expr)" as Expr, expr: Expr = binding.expr.clone());

    Stmt::Decl(swc_core::ecma::ast::Decl::Var(Box::new(VarDecl {
      kind: VarDeclKind::Const,
      decls: vec![VarDeclarator {
        span: Span::default(),
        name: Pat::Ident(swc_core::ecma::ast::BindingIdent::from(
          binding.var_ident.clone(),
        )),
        init: Some(Box::new(wrapped_expr)),
        definite: false,
      }],
      declare: false,
      span: Span::default(),
      ctxt: Default::default(),
    })))
  }
}

impl VisitMut for IdentifierReplacementVisitor {
  fn visit_mut_block_stmt(&mut self, n: &mut BlockStmt) {
    if !self.is_reused_inline_requires_enabled {
      return;
    }

    // Enter new scope
    self.scope_stack.push(ScopeContext::default());

    // Analyze usage patterns in this scope
    self.analyze_scope(n);

    // Process statements one by one, creating reuse bindings as needed
    let mut new_stmts = Vec::new();
    for mut stmt in std::mem::take(&mut n.stmts) {
      // Find identifiers used in this statement that need reuse bindings
      let mut identifier_finder = IdentifierFinder {
        id_replacement: &self.id_replacement,
        found_ids: Vec::new(),
      };
      stmt.visit_with(&mut identifier_finder);

      // For each identifier, check if we need to create a reuse binding
      for id in identifier_finder.found_ids {
        if self.should_create_reuse_binding(&id) {
          if let Some(binding) = self.create_reuse_binding(&id) {
            new_stmts.push(self.create_reuse_declaration(&binding));
          }
        }
      }

      // Process the statement itself
      stmt.visit_mut_with(self);
      new_stmts.push(stmt);
    }

    n.stmts = new_stmts;

    // Exit scope
    self.scope_stack.pop();
  }

  fn visit_mut_expr(&mut self, n: &mut Expr) {
    let Expr::Ident(ident) = n else {
      n.visit_mut_children_with(self);
      return;
    };

    if self.is_reused_inline_requires_enabled {
      let id = ident.to_id();

      // Check if this identifier should be replaced
      if !self.id_replacement.contains_key(&id) {
        n.visit_mut_children_with(self);
        return;
      }

      // Check if there's a reuse binding available
      if let Some(reuse_binding) = self.find_reuse_binding(&id) {
        *n = Expr::Ident(reuse_binding.var_ident.clone());
        return;
      }

      // Otherwise directly insert the require expression
      let replacement_expression = &self.id_replacement[&id];

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

/// Helper to find identifiers that need replacement
struct IdentifierFinder<'a> {
  id_replacement: &'a IndexMap<Id, Expr>,
  found_ids: Vec<Id>,
}

impl<'a> Visit for IdentifierFinder<'a> {
  fn visit_expr(&mut self, n: &Expr) {
    if let Expr::Ident(ident) = n {
      let id = ident.to_id();
      if self.id_replacement.contains_key(&id) && !self.found_ids.contains(&id) {
        self.found_ids.push(id);
      }
    }
    n.visit_children_with(self);
  }
}
