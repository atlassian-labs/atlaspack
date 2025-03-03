mod inlining_visitor;

use crate::inlining_visitor::IdentifierReplacementVisitor;
use std::collections::HashSet;
use swc_core::atoms::atom;
use swc_core::atoms::Atom;
use swc_core::common::Mark;
use swc_core::common::Span;
use swc_core::ecma::ast::Decl;
use swc_core::ecma::ast::EmptyStmt;
use swc_core::ecma::ast::ModuleItem;
use swc_core::ecma::ast::Stmt;
use swc_core::ecma::ast::{CallExpr, Expr, Id, Ident, Lit, VarDecl, VarDeclarator};
use swc_core::ecma::utils::ExprExt;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};

/// Represents a `const i = require('module-id')` statement that has been found.
#[derive(Debug)]
pub struct RequireInitializer {
  /// The variable `i` swc [`Id`] for matching it respecting scope
  pub variable_id: Id,
  /// The imported package atom `'module-id'`
  pub imported_package: Atom,
  /// The entire `require(...)` call for replacement
  pub call_expr: CallExpr,
}

/// Default match patterns for parcel
fn default_require_matchers() -> Vec<RequireMatcher> {
  vec![
    RequireMatcher::Global(atom!("require")),
    RequireMatcher::Keyword(atom!("parcelRequire")),
  ]
}

/// Default ignore patterns for parcel
fn default_ignore_patterns() -> Vec<IgnorePattern> {
  vec![IgnorePattern::IdentifierSymbol(atom!("parcelHelpers"))]
}

/// Extract [RequireInitializer] information from a declarator or return None.
fn match_require_initializer(
  decl: &VarDeclarator,
  unresolved_mark: Mark,
  require_matchers: &[RequireMatcher],
  ignore_patterns: &[IgnorePattern],
) -> Option<RequireInitializer> {
  let expr = decl.init.as_ref()?;
  let call_expr = expr.as_call()?;
  let function_ident = call_expr
    .callee
    .as_expr()
    .and_then(|expr| expr.as_ident())?;

  if !require_matchers
    .iter()
    .any(|matcher| matcher.test(unresolved_mark, function_ident))
  {
    return None;
  }

  let Lit::Str(literal) = call_expr.args[0].expr.as_lit()? else {
    return None;
  };
  let variable_identifier = &decl.name.as_ident()?.id;

  if ignore_patterns
    .iter()
    .any(|pattern| pattern.test(variable_identifier, &literal.value))
  {
    return None;
  }

  let variable_id = variable_identifier.to_id();

  Some(RequireInitializer {
    variable_id,
    imported_package: literal.value.clone(),
    call_expr: call_expr.clone(),
  })
}

/// If this is a parcel interopDefault declaration, return the `Id` of the binding.
///
/// We will recursively inline it.
///
/// This is to handle:
/// ```skip
/// const app = require('x');
/// const appDefault = parcelHelpers.interopDefault(app);
/// ```
///
/// The return value is `appDefault`'s identifier.
fn match_parcel_default_initializer(decl: &VarDeclarator) -> Option<Id> {
  let initializer = decl.init.as_ref()?;
  let binding = decl.name.as_ident()?.id.to_id();
  let call_expr = initializer.as_call()?;
  let callee = call_expr.callee.as_expr()?;
  let callee_object = callee.as_expr().as_member()?;
  let object = callee_object.obj.as_ident()?;
  let property = callee_object.prop.as_ident()?;

  if object.sym == atom!("parcelHelpers") && property.sym == atom!("interopDefault") {
    return Some(binding);
  }

  None
}

/// Different ways to find a `require` call, either using a scope aware `Id` or trying to match
/// against a global symbol.
#[derive(Clone)]
pub enum RequireMatcher {
  Id(Id),
  Global(Atom),
  Keyword(Atom),
}

impl RequireMatcher {
  fn test(&self, unresolved_mark: Mark, ident: &Ident) -> bool {
    match self {
      RequireMatcher::Id(id) => ident.to_id() == *id,
      RequireMatcher::Global(atom) => ident.ctxt.outer() == unresolved_mark && ident.sym == *atom,
      RequireMatcher::Keyword(atom) => ident.sym == *atom,
    }
  }
}

/// Different ways to ignore a `require` call, either using the binding identifier or module-ids.
#[derive(Clone)]
pub enum IgnorePattern {
  IdentifierSymbol(Atom),
  ModuleId(Atom),
  ModuleIdHashSet(HashSet<Atom>),
}

impl IgnorePattern {
  fn test(&self, ident: &Ident, module_id: &Atom) -> bool {
    match self {
      IgnorePattern::IdentifierSymbol(value) => ident.sym == *value,
      IgnorePattern::ModuleId(value) => module_id == value,
      IgnorePattern::ModuleIdHashSet(value) => value.contains(module_id),
    }
  }
}

/// Internal state of the current module stack.
/// Holds the scope aware ids of `require` statements if they are overridden by a `defineModule`
/// style wrapper.
struct ModuleScopeInfo {
  require_matcher: RequireMatcher,
}

/// Builder pattern to build optimizer with defaults
pub struct InlineRequiresOptimizerBuilder {
  unresolved_mark: Mark,
  require_matchers: Vec<RequireMatcher>,
  ignore_patterns: Vec<IgnorePattern>,
}

impl Default for InlineRequiresOptimizerBuilder {
  fn default() -> Self {
    Self {
      unresolved_mark: Default::default(),
      require_matchers: default_require_matchers(),
      ignore_patterns: default_ignore_patterns(),
    }
  }
}

impl InlineRequiresOptimizerBuilder {
  pub fn unresolved_mark(mut self, mark: Mark) -> Self {
    self.unresolved_mark = mark;
    self
  }

  pub fn override_default_require_matchers(
    mut self,
    require_matchers: Vec<RequireMatcher>,
  ) -> Self {
    self.require_matchers = require_matchers;
    self
  }

  pub fn override_default_ignore_patterns(mut self, ignore_patterns: Vec<IgnorePattern>) -> Self {
    self.ignore_patterns = ignore_patterns;
    self
  }

  pub fn add_require_matcher(mut self, require_matcher: RequireMatcher) -> Self {
    self.require_matchers.push(require_matcher);
    self
  }

  pub fn add_ignore_pattern(mut self, ignore_pattern: IgnorePattern) -> Self {
    self.ignore_patterns.push(ignore_pattern);
    self
  }

  pub fn build(self) -> InlineRequiresOptimizer {
    InlineRequiresOptimizer {
      unresolved_mark: self.unresolved_mark,
      require_matchers: self.require_matchers,
      ignore_patterns: self.ignore_patterns,
      ..Default::default()
    }
  }
}

pub struct InlineRequiresCollector {
  unresolved_mark: Mark,
  require_matchers: Vec<RequireMatcher>,
  module_stack: Vec<ModuleScopeInfo>,
  require_initializers: Vec<RequireInitializer>,
  ignore_patterns: Vec<IgnorePattern>,
  identifier_replacement_visitor: IdentifierReplacementVisitor,
}

impl InlineRequiresCollector {
  fn new(
    unresolved_mark: Mark,
    require_matchers: Vec<RequireMatcher>,
    ignore_patterns: Vec<IgnorePattern>,
  ) -> Self {
    InlineRequiresCollector {
      unresolved_mark,
      require_matchers,
      ignore_patterns,
      module_stack: vec![],
      require_initializers: vec![],
      identifier_replacement_visitor: Default::default(),
    }
  }
}

impl VisitMut for InlineRequiresCollector {
  fn visit_mut_expr(&mut self, node: &mut Expr) {
    match node {
      Expr::Fn(fn_expr) => {
        if fn_expr.function.params.len() < 3 {
          node.visit_mut_children_with(self);
          return;
        }

        let (Some(require_ident), Some(module_ident), Some(exports_ident)) = (
          fn_expr.function.params[0].pat.as_ident(),
          fn_expr.function.params[1].pat.as_ident(),
          fn_expr.function.params[2].pat.as_ident(),
        ) else {
          node.visit_mut_children_with(self);
          return;
        };

        if require_ident.sym == atom!("require")
          && module_ident.sym == atom!("module")
          && exports_ident.sym == atom!("exports")
        {
          self.module_stack.push(ModuleScopeInfo {
            require_matcher: RequireMatcher::Id(require_ident.to_id()),
          });
          fn_expr.visit_mut_children_with(self);
          let _ = self.module_stack.pop();
        }
      }
      _ => {
        node.visit_mut_children_with(self);
      }
    }
  }

  fn visit_mut_var_decl(&mut self, node: &mut VarDecl) {
    for decl in node.decls.iter_mut() {
      let mut require_matchers = self.require_matchers.clone();
      if let Some(module_stack_info) = self.module_stack.last() {
        require_matchers.push(module_stack_info.require_matcher.clone());
      }

      if let Some(default_initializer_id) = match_parcel_default_initializer(decl) {
        // first let the normal replacement run on this expression so we inline the require
        decl.visit_mut_children_with(self);
        // get the value we've replaced and carry it forward, we'll inline this value now
        let Some(init) = &decl.init else {
          continue;
        };

        let init = init.as_expr().clone();
        self
          .identifier_replacement_visitor
          .add_replacement(default_initializer_id, init);

        continue;
      }

      let Some(initializer) = match_require_initializer(
        decl,
        self.unresolved_mark,
        &require_matchers,
        &self.ignore_patterns,
      ) else {
        decl.visit_mut_children_with(self);
        continue;
      };

      self.identifier_replacement_visitor.add_replacement(
        initializer.variable_id.clone(),
        Expr::Call(initializer.call_expr.clone()),
      );
      self.require_initializers.push(initializer);
    }
  }
}

pub struct InlineRequiresReplacer {
  unresolved_mark: Mark,
  require_matchers: Vec<RequireMatcher>,
  ignore_patterns: Vec<IgnorePattern>,
  identifier_replacement_visitor: IdentifierReplacementVisitor,
}

impl InlineRequiresReplacer {
  fn new(
    unresolved_mark: Mark,
    require_matchers: Vec<RequireMatcher>,
    ignore_patterns: Vec<IgnorePattern>,
    identifier_replacement_visitor: IdentifierReplacementVisitor,
  ) -> Self {
    InlineRequiresReplacer {
      unresolved_mark,
      require_matchers,
      ignore_patterns,
      identifier_replacement_visitor,
    }
  }
}

impl VisitMut for InlineRequiresReplacer {
  fn visit_mut_expr(&mut self, node: &mut Expr) {
    self.identifier_replacement_visitor.visit_mut_expr(node);
    node.visit_mut_children_with(self);
  }

  fn visit_mut_var_decl(&mut self, node: &mut VarDecl) {
    node.decls.retain_mut(|decl| {
      if match_parcel_default_initializer(decl).is_some() {
        // first let the normal replacement run on this expression so we inline the require
        decl.visit_mut_children_with(self);
        // If this variable is actually initialized, then we can remove it
        return decl.init.is_none();
      }

      // Only retain if it's not one of Atlaspack's initiatlizers
      match_require_initializer(
        decl,
        self.unresolved_mark,
        &self.require_matchers,
        &self.ignore_patterns,
      )
      .is_none()
    })
  }

  fn visit_mut_stmt(&mut self, stmt: &mut Stmt) {
    stmt.visit_mut_children_with(self);

    if let Stmt::Decl(Decl::Var(var)) = stmt {
      if var.decls.is_empty() {
        *stmt = Stmt::Empty(EmptyStmt {
          span: Span::default(),
        });
      }
    }
  }

  fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
    stmts.visit_mut_children_with(self);
    stmts.retain(|s| !matches!(s, Stmt::Empty(..)));
  }

  fn visit_mut_module_items(&mut self, stmts: &mut Vec<ModuleItem>) {
    stmts.retain(|s| !matches!(s, ModuleItem::Stmt(Stmt::Empty(..))));
  }
}

/// Inlines require statements in module definitions.
///
/// Use `InlineRequiresOptimizer::builder()` to construct instances.
///
/// You may add ignore patterns to skip certain modules or variable identifier bindings.
///
/// You may add require matchers to match against certain function names or Ids for `require`.
///
/// The `unresolved_mark` must be set to respect scope and not replace any `require` variable in
/// the module that might not be relevant.
///
/// Defaults can be overridden (do not match on `require`, do not ignore `parcelHelpers`).
///
/// After replacement has been executed, `InlineRequiresOptimizer::require_initializers()` may be
/// used to retrieve which statements have been matched against. This would be used for diagnostics
/// purposes only.
///
/// The replacements are wrapped with `(0, $expr)`. This is to avoid issues when rewriting
/// `new ...` expressions, where inserting a bare function like symbol will cause different
/// treatment when instantiating classes. See [`IdentifierReplacementVisitor`].
#[non_exhaustive]
pub struct InlineRequiresOptimizer {
  unresolved_mark: Mark,
  require_matchers: Vec<RequireMatcher>,
  require_initializers: Vec<RequireInitializer>,
  ignore_patterns: Vec<IgnorePattern>,
}

impl Default for InlineRequiresOptimizer {
  fn default() -> Self {
    InlineRequiresOptimizer {
      unresolved_mark: Default::default(),
      require_matchers: default_require_matchers(),
      ignore_patterns: default_ignore_patterns(),
      require_initializers: vec![],
    }
  }
}

impl InlineRequiresOptimizer {
  /// Get the results for what initializers have been replaced
  pub fn require_initializers(&self) -> &[RequireInitializer] {
    &self.require_initializers
  }

  pub fn builder() -> InlineRequiresOptimizerBuilder {
    InlineRequiresOptimizerBuilder::default()
  }
}

impl VisitMut for InlineRequiresOptimizer {
  fn visit_mut_module_items(&mut self, stmts: &mut Vec<ModuleItem>) {
    let mut collector_visitor = InlineRequiresCollector::new(
      self.unresolved_mark,
      self.require_matchers.clone(),
      self.ignore_patterns.clone(),
    );

    stmts.visit_mut_children_with(&mut collector_visitor);

    let mut replacer_visitor = InlineRequiresReplacer::new(
      collector_visitor.unresolved_mark,
      collector_visitor.require_matchers,
      collector_visitor.ignore_patterns,
      collector_visitor.identifier_replacement_visitor,
    );

    self.require_initializers = collector_visitor.require_initializers;

    // Needs to start with self to determine whether to retain module items
    stmts.visit_mut_with(&mut replacer_visitor);
  }
}

#[cfg(test)]
mod tests {
  use atlaspack_swc_runner::test_utils::{run_test_visit, RunVisitResult};
  use pretty_assertions::assert_eq;

  use super::*;

  #[test]
  fn it_inlines_require_statements_that_are_declared_later() {
    let code = r#"
parcelRegister("k4tEj", function(module, exports) {
  Object.defineProperty(module.exports, "InternSet", {
    enumerable: true,
    get: function() {
        return $g34Jm.InternSet;
    }
  });

  var $g34Jm = require("internmap");
});
    "#
    .trim();
    let RunVisitResult { output_code, .. } = run_test_visit(code, |ctx| InlineRequiresOptimizer {
      unresolved_mark: ctx.unresolved_mark,
      ..Default::default()
    });

    let expected_output = r#"
parcelRegister("k4tEj", function(module, exports) {
  Object.defineProperty(module.exports, "InternSet", {
    enumerable: true,
    get: function() {
        return (0, require('internmap')).InternSet;
    }
  });
});
    "#
    .trim();
    assert_eq!(output_code.trim(), expected_output);
  }

  #[test]
  fn it_respects_variables_across_scopes() {
    let code = r#"
parcelRegister("k4tEj", function(module, exports) {
  Object.defineProperty(module.exports, "InternSet", {
    enumerable: true,
    get: function() {
        return $g34Jm.InternSet;
    }
  });

  var $g34Jm = require("internmap");
});

parcelRegister("12345", function(module, exports) {
  var testVar = $g34Jm.otherKey;
  console.log(testVar);
});
    "#
    .trim();
    let RunVisitResult { output_code, .. } = run_test_visit(code, |ctx| InlineRequiresOptimizer {
      unresolved_mark: ctx.unresolved_mark,
      ..Default::default()
    });

    let expected_output = r#"
parcelRegister("k4tEj", function(module, exports) {
  Object.defineProperty(module.exports, "InternSet", {
    enumerable: true,
    get: function() {
        return (0, require('internmap')).InternSet;
    }
  });
});

parcelRegister("12345", function(module, exports) {
  var testVar = $g34Jm.otherKey;
  console.log(testVar);
});
    "#
    .trim();
    assert_eq!(output_code.trim(), expected_output);
  }

  #[test]
  fn it_inlines_require_statements_in_simple_commonjs_modules() {
    let code = r#"
const fs = require('fs');
function doWork() {
    return fs.readFileSync('./something');
}
    "#
    .trim();

    let RunVisitResult { output_code, .. } = run_test_visit(code, |ctx| InlineRequiresOptimizer {
      unresolved_mark: ctx.unresolved_mark,
      ..Default::default()
    });

    let expected_output = r#"
function doWork() {
    return (0, require('fs')).readFileSync('./something');
}
    "#
    .trim();
    assert_eq!(output_code.trim(), expected_output);
  }

  #[test]
  fn is_inlines_require_statements_in_parcel_module_wrappers() {
    let code = r#"
parcelRequire.register('moduleId', function(require, module, exports) {

    const fs = require('fs');
    function doWork() {
        return fs.readFileSync('./something');
    }

});
    "#
    .trim();

    let RunVisitResult { output_code, .. } = run_test_visit(code, |ctx| InlineRequiresOptimizer {
      unresolved_mark: ctx.unresolved_mark,
      ..Default::default()
    });

    let expected_output = r#"
parcelRequire.register('moduleId', function(require, module, exports) {
    function doWork() {
        return (0, require('fs')).readFileSync('./something');
    }
});
    "#
    .trim();
    assert_eq!(output_code.trim(), expected_output);
  }

  #[test]
  fn ignores_parcel_helpers_require_statements() {
    let code = r#"
const parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
    "#
    .trim();

    let RunVisitResult { output_code, .. } = run_test_visit(code, |ctx| InlineRequiresOptimizer {
      unresolved_mark: ctx.unresolved_mark,
      ..Default::default()
    });

    let expected_output = r#"
const parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
    "#
    .trim();
    assert_eq!(output_code.trim(), expected_output);
  }

  #[test]
  fn ignores_modules_that_are_on_the_ignore_list() {
    let code = r#"
const sideEffects = require("side-effects");
    "#
    .trim();

    let RunVisitResult { output_code, .. } = run_test_visit(code, |ctx| InlineRequiresOptimizer {
      unresolved_mark: ctx.unresolved_mark,
      ignore_patterns: vec![IgnorePattern::ModuleId(atom!("side-effects"))],
      ..Default::default()
    });

    let expected_output = r#"
const sideEffects = require("side-effects");
    "#
    .trim();
    assert_eq!(output_code.trim(), expected_output);
  }

  #[test]
  fn handles_interop_default_calls() {
    let code = r#"
const app = require("./App");
const appDefault = parcelHelpers.interopDefault(app);

function run() {
    return appDefault.test();
}
    "#
    .trim();

    let RunVisitResult { output_code, .. } = run_test_visit(code, |ctx| InlineRequiresOptimizer {
      unresolved_mark: ctx.unresolved_mark,
      ..Default::default()
    });

    let expected_output = r#"
function run() {
    return (0, parcelHelpers.interopDefault((0, require("./App")))).test();
}
    "#
    .trim();
    assert_eq!(output_code.trim(), expected_output);
  }
}
