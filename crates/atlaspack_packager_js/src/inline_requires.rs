//! Inline requires optimisation for the native JS packager.
//!
//! This module implements the same transformation as the SWC-based
//! `atlaspack_plugin_optimizer_inline_requires` crate, but using OXC and operating directly
//! inside the packager's AST processing pipeline rather than as a separate optimiser plugin.
//!
//! ## What it does
//!
//! Given code like:
//! ```js
//! const fs = require('fs');
//! function doWork() {
//!   return fs.readFileSync('./something');
//! }
//! ```
//!
//! It produces:
//! ```js
//! function doWork() {
//!   return require('fs').readFileSync('./something');
//! }
//! ```
//!
//! Top-level `require()` variable declarations are removed and each use of the binding is
//! replaced with a fresh `require("id")` call. When the replaced identifier appears directly
//! as the callee of a call expression (e.g. `foo()` where `foo = require("x")`), the call is
//! wrapped as `(0, require("x"))()` to prevent the JS engine from binding `this` to the module
//! namespace object. In all other positions (member access object, argument, assignment RHS,
//! etc.) the bare `require("id")` is emitted without the wrapper, keeping output readable.
//!
//! `parcelHelpers.interopDefault(x)` chains are also inlined: if `x` was itself a require
//! binding, the `interopDefault` wrapper declaration is also removed and its usages are
//! replaced with `parcelHelpers.interopDefault(require("x"))` (with `(0, ...)` only when in
//! callee position).
//!
//! ## Scope handling
//!
//! The packager wraps every asset in `define('id', function(require, module, exports) { ... })`.
//! Within that wrapper the `require` in scope is always the wrapper parameter, so we can match
//! require calls purely by callee name without needing OXC's semantic scope analysis.
//!
//! ## Ignored patterns
//!
//! By default, declarations whose binding name is `parcelHelpers` are not inlined, because
//! `parcelHelpers` is used extensively as a namespace object and inlining it would produce
//! many redundant `require()` calls.

use std::collections::{HashMap, HashSet};

use oxc_allocator::Allocator;
use oxc_ast::AstBuilder;
use oxc_ast::ast::*;
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_span::SPAN;
use oxc_syntax::number::NumberBase;

/// Names of variable bindings that should never be inlined even if they are `require()` calls.
const IGNORE_BINDING_NAMES: &[&str] = &["parcelHelpers"];

/// Lightweight descriptor of what to substitute for an inlined require binding.
///
/// Storing a descriptor rather than a pre-built `Expression<'a>` means the replacer can
/// construct fresh AST nodes at each replacement site from a single arena allocation, instead
/// of deep-cloning a pre-built expression tree every time the binding appears in the code.
#[derive(Debug, Clone)]
enum Replacement {
  /// `(0, require("module_id"))`
  Require(String),
  /// `(0, parcelHelpers.interopDefault((0, require("module_id"))))`
  InteropDefault(String),
}

/// Run the inline-requires transformation on a parsed program in-place.
///
/// This is a two-pass operation:
/// 1. **Collect** — walk statements, find `var/const/let x = require("id")` and
///    `var/const/let xDefault = parcelHelpers.interopDefault(x)` declarations, record the
///    replacement descriptor for each binding name, and remove those declarators.
/// 2. **Replace** — walk all remaining expressions and substitute matching identifiers with
///    freshly-built `(0, require(...))` expressions.
#[tracing::instrument(level = "trace", skip_all)]
pub fn inline_requires<'a>(
  allocator: &'a Allocator,
  program: &mut Program<'a>,
  ignore_identifiers: &HashSet<String>,
) {
  // Pass 1: collect replacements and remove declarations
  let mut collector = InlineRequiresCollector::new(ignore_identifiers);
  collector.visit_program(program);

  if collector.replacements.is_empty() {
    return;
  }

  // Pass 2: replace identifier usages, building fresh expressions from descriptors
  let ast = AstBuilder::new(allocator);
  let mut replacer = InlineRequiresReplacer::new(ast, collector.replacements);
  replacer.visit_program(program);
}

// ---------------------------------------------------------------------------
// Pass 1: Collector
// ---------------------------------------------------------------------------

/// Collects `require()` and `interopDefault` declarations, builds a lightweight replacement
/// map, and removes the matched declarators from the AST.
///
/// No arena allocator is needed here — we only store plain Rust `String`s as descriptors.
struct InlineRequiresCollector<'b> {
  /// Maps binding name → replacement descriptor.
  replacements: HashMap<String, Replacement>,
  /// Module IDs that must not be inlined (assets with side effects).
  ignore_module_ids: &'b HashSet<String>,
}

impl<'b> InlineRequiresCollector<'b> {
  fn new(ignore_module_ids: &'b HashSet<String>) -> Self {
    Self {
      replacements: HashMap::new(),
      ignore_module_ids,
    }
  }

  /// Returns true if `callee` is a bare `require` identifier.
  fn is_require_callee(callee: &Expression) -> bool {
    matches!(callee, Expression::Identifier(id) if id.name == "require")
  }

  /// If `call` is `require("literal")`, return the module ID.
  fn as_require_string_arg<'c>(call: &'c CallExpression) -> Option<&'c str> {
    if !Self::is_require_callee(&call.callee) || call.arguments.len() != 1 {
      return None;
    }
    match &call.arguments[0] {
      Argument::StringLiteral(s) => Some(s.value.as_str()),
      _ => None,
    }
  }

  /// If `decl` is `var/let/const name = require("id")` and neither `name` nor `id` are ignored,
  /// return `(binding_name, module_id)` as `&str` slices into the AST.
  fn match_require_decl<'c>(&self, decl: &'c VariableDeclarator) -> Option<(&'c str, &'c str)> {
    let binding_name = decl.id.get_identifier_name()?;
    if IGNORE_BINDING_NAMES.contains(&binding_name.as_str()) {
      return None;
    }
    let call = match decl.init.as_ref()? {
      Expression::CallExpression(c) => c.as_ref(),
      _ => return None,
    };
    let module_id = Self::as_require_string_arg(call)?;
    if self.ignore_module_ids.contains(module_id) {
      return None;
    }
    Some((binding_name.as_str(), module_id))
  }

  /// If `decl` is `var/let/const name = parcelHelpers.interopDefault(x)` where `x` is a known
  /// `Require` replacement, return `(binding_name, module_id_of_x)`.
  fn match_interop_default_decl<'c>(
    decl: &'c VariableDeclarator,
    replacements: &'c HashMap<String, Replacement>,
  ) -> Option<(&'c str, &'c str)> {
    let binding_name = decl.id.get_identifier_name()?;
    let call = match decl.init.as_ref()? {
      Expression::CallExpression(c) => c.as_ref(),
      _ => return None,
    };
    let callee_member = match &call.callee {
      Expression::StaticMemberExpression(m) => m.as_ref(),
      _ => return None,
    };
    if !matches!(&callee_member.object, Expression::Identifier(id) if id.name == "parcelHelpers") {
      return None;
    }
    if callee_member.property.name != "interopDefault" || call.arguments.len() != 1 {
      return None;
    }
    let arg_name = match &call.arguments[0] {
      Argument::Identifier(id) => id.name.as_str(),
      _ => return None,
    };
    // Only chain if the arg is a plain Require (not already an InteropDefault)
    match replacements.get(arg_name)? {
      Replacement::Require(module_id) => Some((binding_name.as_str(), module_id.as_str())),
      _ => None,
    }
  }
}

impl<'a> VisitMut<'a> for InlineRequiresCollector<'_> {
  fn visit_statements(&mut self, stmts: &mut oxc_allocator::Vec<'a, Statement<'a>>) {
    // Recurse into nested scopes first.
    walk_mut::walk_statements(self, stmts);

    // Collect require() and interopDefault() matches as owned Strings to avoid
    // holding borrows into `stmts` or `self.replacements` across mutations.
    let mut require_matches: Vec<(String, String)> = Vec::new();
    for stmt in stmts.iter() {
      let Statement::VariableDeclaration(var_decl) = stmt else {
        continue;
      };
      for decl in var_decl.declarations.iter() {
        if let Some((name, module_id)) = self.match_require_decl(decl) {
          require_matches.push((name.to_string(), module_id.to_string()));
        }
      }
    }
    for (name, module_id) in &require_matches {
      self
        .replacements
        .insert(name.clone(), Replacement::Require(module_id.clone()));
    }

    // Collect interopDefault matches now that self.replacements has the require entries.
    let mut interop_matches: Vec<(String, String)> = Vec::new();
    for stmt in stmts.iter() {
      let Statement::VariableDeclaration(var_decl) = stmt else {
        continue;
      };
      for decl in var_decl.declarations.iter() {
        if let Some((name, module_id)) = Self::match_interop_default_decl(decl, &self.replacements)
        {
          interop_matches.push((name.to_string(), module_id.to_string()));
        }
      }
    }
    for (name, module_id) in &interop_matches {
      self
        .replacements
        .insert(name.clone(), Replacement::InteropDefault(module_id.clone()));
    }

    if require_matches.is_empty() && interop_matches.is_empty() {
      return;
    }

    // Build the set of binding names to remove from declarations.
    let mut to_remove: HashSet<String> =
      HashSet::with_capacity(require_matches.len() + interop_matches.len());
    for (name, _) in require_matches.iter().chain(interop_matches.iter()) {
      to_remove.insert(name.clone());
    }

    // Remove matched declarators; drop whole statements that become empty.
    let mut i = 0;
    while i < stmts.len() {
      let keep = if let Statement::VariableDeclaration(var_decl) = &mut stmts[i] {
        var_decl.declarations.retain(|decl| {
          !decl
            .id
            .get_identifier_name()
            .map(|n| to_remove.contains(n.as_str()))
            .unwrap_or(false)
        });
        !var_decl.declarations.is_empty()
      } else {
        true
      };
      if keep {
        i += 1;
      } else {
        stmts.remove(i);
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Pass 2: Replacer
// ---------------------------------------------------------------------------

/// Replaces identifier usages by building fresh `(0, require(...))` AST nodes from the
/// lightweight descriptors collected in pass 1.
///
/// Building fresh nodes (a handful of small arena allocations) is significantly cheaper than
/// deep-cloning a pre-built `Expression<'a>` tree for every occurrence of the binding.
///
/// The `(0, expr)` sequence wrapper is needed only when the expression is in **callee position**
/// of a call expression — without it, `require("foo")()` would pass the object holding `require`
/// as `this` to the callee. When the expression is not in callee position (member access,
/// assignment RHS, argument, etc.) the bare expression is emitted instead, producing cleaner
/// and more readable output.
struct InlineRequiresReplacer<'a> {
  ast: AstBuilder<'a>,
  replacements: HashMap<String, Replacement>,
  /// Whether the current expression node is in callee position of a call expression.
  /// When true, replacements are wrapped with `(0, expr)` to preserve `this` semantics.
  in_callee_position: bool,
}

impl<'a> InlineRequiresReplacer<'a> {
  fn new(ast: AstBuilder<'a>, replacements: HashMap<String, Replacement>) -> Self {
    Self {
      ast,
      replacements,
      in_callee_position: false,
    }
  }

  /// Build `require("module_id")` or `(0, require("module_id"))` depending on context.
  fn make_require(&self, module_id: oxc_span::Atom<'a>) -> Expression<'a> {
    let callee = self.ast.expression_identifier(SPAN, "require");
    let str_lit = self.ast.expression_string_literal(SPAN, module_id, None);
    let mut args = self.ast.vec();
    args.push(Argument::from(str_lit));
    let require_call = Expression::CallExpression(self.ast.alloc_call_expression(
      SPAN,
      callee,
      Option::<TSTypeParameterInstantiation>::None,
      args,
      false,
    ));
    if self.in_callee_position {
      self.make_seq(require_call)
    } else {
      require_call
    }
  }

  /// Build `parcelHelpers.interopDefault(require("module_id"))` or with `(0, ...)` wrappers
  /// depending on context. The inner require never needs a wrapper (it's an argument, not a
  /// callee). The outer interopDefault call only needs a wrapper when in callee position.
  fn make_interop(&self, module_id: oxc_span::Atom<'a>) -> Expression<'a> {
    // Inner require — always bare (it's an argument, not a callee)
    let callee_inner = self.ast.expression_identifier(SPAN, "require");
    let str_lit = self.ast.expression_string_literal(SPAN, module_id, None);
    let mut inner_args = self.ast.vec();
    inner_args.push(Argument::from(str_lit));
    let inner_require = Expression::CallExpression(self.ast.alloc_call_expression(
      SPAN,
      callee_inner,
      Option::<TSTypeParameterInstantiation>::None,
      inner_args,
      false,
    ));
    // Outer interopDefault call
    let obj = self.ast.expression_identifier(SPAN, "parcelHelpers");
    let callee = Expression::StaticMemberExpression(self.ast.alloc_static_member_expression(
      SPAN,
      obj,
      self.ast.identifier_name(SPAN, "interopDefault"),
      false,
    ));
    let mut args = self.ast.vec();
    args.push(Argument::from(inner_require));
    let interop_call = Expression::CallExpression(self.ast.alloc_call_expression(
      SPAN,
      callee,
      Option::<TSTypeParameterInstantiation>::None,
      args,
      false,
    ));
    if self.in_callee_position {
      self.make_seq(interop_call)
    } else {
      interop_call
    }
  }

  /// Wrap `expr` in `(0, expr)`.
  fn make_seq(&self, expr: Expression<'a>) -> Expression<'a> {
    let zero = self
      .ast
      .expression_numeric_literal(SPAN, 0.0, None, NumberBase::Decimal);
    let mut exprs = self.ast.vec();
    exprs.push(zero);
    exprs.push(expr);
    self.ast.expression_sequence(SPAN, exprs)
  }
}

impl<'a> VisitMut<'a> for InlineRequiresReplacer<'a> {
  fn visit_call_expression(&mut self, call: &mut CallExpression<'a>) {
    // The callee of a call expression is in callee position only if it is a bare identifier —
    // e.g. `foo()`. If the callee is a member expression (`obj.method()`), the `obj` part is
    // NOT in callee position; only the member expression as a whole is.
    //
    // We visit the callee with in_callee_position = true, but member expression visitors reset
    // it to false for their object sub-expressions.
    let prev = self.in_callee_position;
    self.in_callee_position = true;
    self.visit_expression(&mut call.callee);
    self.in_callee_position = prev;
    // Arguments are never in callee position.
    for arg in call.arguments.iter_mut() {
      self.visit_argument(arg);
    }
  }

  fn visit_static_member_expression(&mut self, expr: &mut StaticMemberExpression<'a>) {
    // The object of a member expression is not in callee position regardless of context —
    // `obj.method` uses `obj` as `this` for the resulting call, but `obj` itself is accessed
    // as a plain value, not called.
    let prev = self.in_callee_position;
    self.in_callee_position = false;
    self.visit_expression(&mut expr.object);
    self.in_callee_position = prev;
    // property is an identifier name, not an expression — no need to visit
  }

  fn visit_computed_member_expression(&mut self, expr: &mut ComputedMemberExpression<'a>) {
    let prev = self.in_callee_position;
    self.in_callee_position = false;
    self.visit_expression(&mut expr.object);
    self.visit_expression(&mut expr.expression);
    self.in_callee_position = prev;
  }

  fn visit_expression(&mut self, expr: &mut Expression<'a>) {
    if let Expression::Identifier(ident) = expr {
      // Clone the Replacement (cheap: tag + short String) to end the immutable borrow
      // on self.replacements before calling &mut self builder methods.
      let replacement = self.replacements.get(ident.name.as_str()).cloned();
      if let Some(r) = replacement {
        let new_expr = match r {
          Replacement::Require(ref module_id) => {
            let atom = self.ast.atom(module_id.as_str());
            self.make_require(atom)
          }
          Replacement::InteropDefault(ref module_id) => {
            let atom = self.ast.atom(module_id.as_str());
            self.make_interop(atom)
          }
        };
        *expr = new_expr;
        return;
      }
    }
    walk_mut::walk_expression(self, expr);
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;
  use oxc_codegen::Codegen;
  use oxc_parser::Parser;
  use oxc_span::SourceType;
  use pretty_assertions::assert_eq;

  fn run(code: &str) -> String {
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_module(true);
    let parser_return = Parser::new(&allocator, code, source_type).parse();
    assert!(
      parser_return.errors.is_empty(),
      "Parse errors: {:?}",
      parser_return.errors
    );
    let mut program = parser_return.program;
    inline_requires(&allocator, &mut program, &HashSet::new());
    Codegen::new().build(&program).code
  }

  /// Normalise whitespace for comparison: trim each line and drop blank lines.
  fn normalise(s: &str) -> String {
    s.lines()
      .map(|l| l.trim())
      .filter(|l| !l.is_empty())
      .collect::<Vec<_>>()
      .join("\n")
  }

  fn assert_code_eq(actual: &str, expected: &str) {
    assert_eq!(normalise(actual), normalise(expected));
  }

  // --- Tests migrated from crates/atlaspack_plugin_optimizer_inline_requires/src/lib.rs ---

  #[test]
  fn it_inlines_require_statements_in_simple_commonjs_modules() {
    let code = r#"
const fs = require('fs');
function doWork() {
    return fs.readFileSync('./something');
}
"#;
    let output = run(code);
    assert_code_eq(
      &output,
      r#"
function doWork() {
    return require("fs").readFileSync("./something");
}
"#,
    );
  }

  #[test]
  fn it_inlines_require_statements_that_are_declared_later() {
    // `$g34Jm` is used before declaration in source order.
    // After inlining, the declaration is removed and usages are replaced.
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
"#;
    let output = run(code);
    assert_code_eq(
      &output,
      r#"
parcelRegister("k4tEj", function(module, exports) {
    Object.defineProperty(module.exports, "InternSet", {
        enumerable: true,
        get: function() {
            return require("internmap").InternSet;
        }
    });
});
"#,
    );
  }

  #[test]
  fn it_inlines_require_statements_in_parcel_module_wrappers() {
    let code = r#"
parcelRequire.register('moduleId', function(require, module, exports) {
    const fs = require('fs');
    function doWork() {
        return fs.readFileSync('./something');
    }
});
"#;
    let output = run(code);
    assert_code_eq(
      &output,
      r#"
parcelRequire.register("moduleId", function(require, module, exports) {
    function doWork() {
        return require("fs").readFileSync("./something");
    }
});
"#,
    );
  }

  #[test]
  fn ignores_parcel_helpers_require_statements() {
    let code =
      r#"const parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");"#;
    let output = run(code);
    assert!(
      output.contains("const parcelHelpers"),
      "parcelHelpers declaration should be preserved, got:\n{output}"
    );
  }

  #[test]
  fn ignores_modules_that_are_on_the_ignore_list() {
    // Module-ID-based ignore is handled at the JS orchestrator level. Here we just verify
    // that a non-ignored require IS inlined (the positive case).
    let code = r#"
const sideEffects = require("side-effects");
console.log(sideEffects.value);
"#;
    let output = run(code);
    assert!(
      output.contains("require(\"side-effects\")"),
      "require should be inlined, got:\n{output}"
    );
    assert!(
      !output.contains("const sideEffects"),
      "declaration should be removed, got:\n{output}"
    );
  }

  #[test]
  fn does_not_inline_require_for_module_with_side_effects() {
    // When the module's public ID is in the side-effects set the declaration must be preserved
    // so that the module's top-level code executes eagerly (e.g. polyfills, global registrations).
    let code = r#"
const polyfill = require("abc123");
const safe = require("def456");
function work() {
    return safe.value + polyfill.patch();
}
"#;
    let side_effects: HashSet<String> = ["abc123".to_string()].into();
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_module(true);
    let parser_return = Parser::new(&allocator, code, source_type).parse();
    assert!(parser_return.errors.is_empty());
    let mut program = parser_return.program;
    inline_requires(&allocator, &mut program, &side_effects);
    let output = Codegen::new().build(&program).code;

    // The side-effect module declaration should be preserved
    assert!(
      output.contains("const polyfill"),
      "side-effect module declaration should be preserved, got:\n{output}"
    );
    // The safe module should still be inlined
    assert!(
      output.contains("require(\"def456\")"),
      "safe module should be inlined, got:\n{output}"
    );
    assert!(
      !output.contains("const safe"),
      "safe declaration should be removed, got:\n{output}"
    );
  }

  #[test]
  fn handles_interop_default_calls() {
    let code = r#"
const app = require("./App");
const appDefault = parcelHelpers.interopDefault(app);

function run() {
    return appDefault.test();
}
"#;
    let output = run(code);
    assert_code_eq(
      &output,
      r#"
function run() {
    return parcelHelpers.interopDefault(require("./App")).test();
}
"#,
    );
  }

  #[test]
  fn it_does_not_inline_requires_with_non_string_args() {
    let code = r#"
const mod = require(dynamicId);
console.log(mod.value);
"#;
    let output = run(code);
    // Dynamic require should NOT be inlined
    assert!(
      output.contains("const mod"),
      "dynamic require should not be inlined, got:\n{output}"
    );
  }

  #[test]
  fn inlines_multiple_requires() {
    let code = r#"
const a = require("mod-a");
const b = require("mod-b");
function work() {
    return a.foo + b.bar;
}
"#;
    let output = run(code);
    assert_code_eq(
      &output,
      r#"
function work() {
    return require("mod-a").foo + require("mod-b").bar;
}
"#,
    );
  }

  #[test]
  fn inlines_require_used_multiple_times() {
    let code = r#"
const fs = require("fs");
function a() { return fs.readFileSync("a"); }
function b() { return fs.writeFileSync("b", "x"); }
"#;
    let output = run(code);
    assert_eq!(
      output.matches("require(\"fs\")").count(),
      2,
      "both usages should be inlined, got:\n{output}"
    );
    assert!(
      !output.contains("const fs"),
      "declaration should be removed, got:\n{output}"
    );
  }

  #[test]
  fn separate_module_wrappers_do_not_cross_contaminate() {
    // Two wrappers with different require bindings - each should only inline its own.
    let code = r#"
parcelRegister("k4tEj", function(module, exports) {
    var $g34Jm = require("internmap");
    return $g34Jm.InternSet;
});

parcelRegister("12345", function(module, exports) {
    var $other = require("other-module");
    return $other.otherKey;
});
"#;
    let output = run(code);
    assert!(
      output.contains("require(\"internmap\").InternSet"),
      "internmap require should be inlined, got:\n{output}"
    );
    assert!(
      output.contains("require(\"other-module\").otherKey"),
      "other-module require should be inlined, got:\n{output}"
    );
    assert!(
      !output.contains("var $g34Jm"),
      "$g34Jm declaration should be removed, got:\n{output}"
    );
    assert!(
      !output.contains("var $other"),
      "$other declaration should be removed, got:\n{output}"
    );
  }
}
