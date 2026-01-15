use swc_core::ecma::ast::*;
use swc_core::ecma::visit::VisitMut;
use swc_core::ecma::visit::VisitMutWith;

/// Strips `declare const` statements that don't have an initializer.
/// This prevents the resolver from seeing them and marking them as bindings.
/// Statements like `declare const foo: string = "hello";` are kept.
pub struct DeclareConstStripper;

impl DeclareConstStripper {
  /// Efficiently checks if the code contains `declare const` statements.
  /// This is a fast string check to avoid running the full stripper when not needed.
  pub fn has_declare_const(code: &str) -> bool {
    code.contains("declare const")
  }
}

impl VisitMut for DeclareConstStripper {
  fn visit_mut_module(&mut self, module: &mut Module) {
    // Filter out `declare const` statements without initializers
    module.body.retain(|item| {
      match item {
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) if var.declare => {
          // Keep only if at least one declarator has an initializer
          var.decls.iter().any(|decl| decl.init.is_some())
        }
        _ => true,
      }
    });
    module.visit_mut_children_with(self);
  }

  fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
    // Filter out `declare const` statements without initializers from statement lists
    stmts.retain(|stmt| {
      match stmt {
        Stmt::Decl(Decl::Var(var)) if var.declare => {
          // Keep only if at least one declarator has an initializer
          var.decls.iter().any(|decl| decl.init.is_some())
        }
        _ => true,
      }
    });
    stmts.visit_mut_children_with(self);
  }
}
