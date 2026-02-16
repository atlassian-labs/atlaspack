mod inlining_visitor;

use crate::inlining_visitor::IdentifierReplacementVisitor;
use std::collections::HashSet;
use swc_core::atoms::Atom;
use swc_core::atoms::atom;
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
    node.decls.retain_mut(|decl| {
      let mut require_matchers = self.require_matchers.clone();
      if let Some(module_stack_info) = self.module_stack.last() {
        require_matchers.push(module_stack_info.require_matcher.clone());
      }

      if let Some(default_initializer_id) = match_parcel_default_initializer(decl) {
        // first let the normal replacement run on this expression so we inline the require
        decl.visit_mut_children_with(self);
        // get the value we've replaced and carry it forward, we'll inline this value now
        let Some(init) = &decl.init else {
          return true;
        };

        let init = init.as_expr().clone();
        self
          .identifier_replacement_visitor
          .add_replacement(default_initializer_id, init);

        return false;
      }

      let Some(initializer) = match_require_initializer(
        decl,
        self.unresolved_mark,
        &require_matchers,
        &self.ignore_patterns,
      ) else {
        decl.visit_mut_children_with(self);
        return true;
      };

      self.identifier_replacement_visitor.add_replacement(
        initializer.variable_id.clone(),
        Expr::Call(initializer.call_expr.clone()),
      );
      self.require_initializers.push(initializer);

      false
    });
  }
}

pub struct InlineRequiresReplacer {
  identifier_replacement_visitor: IdentifierReplacementVisitor,
}

impl InlineRequiresReplacer {
  fn new(identifier_replacement_visitor: IdentifierReplacementVisitor) -> Self {
    InlineRequiresReplacer {
      identifier_replacement_visitor,
    }
  }
}

impl VisitMut for InlineRequiresReplacer {
  fn visit_mut_expr(&mut self, node: &mut Expr) {
    self.identifier_replacement_visitor.visit_mut_expr(node);
    node.visit_mut_children_with(self);
  }

  fn visit_mut_stmt(&mut self, stmt: &mut Stmt) {
    stmt.visit_mut_children_with(self);

    if let Stmt::Decl(Decl::Var(var)) = stmt
      && var.decls.is_empty()
    {
      *stmt = Stmt::Empty(EmptyStmt {
        span: Span::default(),
      });
    }
  }

  fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
    stmts.visit_mut_children_with(self);
    stmts.retain(|s| !matches!(s, Stmt::Empty(..)));
  }

  fn visit_mut_module_items(&mut self, stmts: &mut Vec<ModuleItem>) {
    stmts.visit_mut_children_with(self);
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

    let mut replacer_visitor =
      InlineRequiresReplacer::new(collector_visitor.identifier_replacement_visitor);

    self.require_initializers = collector_visitor.require_initializers;

    stmts.visit_mut_with(&mut replacer_visitor);
  }
}

#[cfg(test)]
mod tests {
  use atlaspack_swc_runner::test_utils::{RunVisitResult, run_test_visit};
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
            return (0, require("internmap")).InternSet;
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
            return (0, require("internmap")).InternSet;
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

  /// Helper: decode a base64-VLQ value from the mappings string.
  /// Returns (value, bytes_consumed).
  fn decode_vlq(mappings: &[u8], start: usize) -> (i64, usize) {
    const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut value: i64 = 0;
    let mut shift = 0u32;
    let mut i = start;
    loop {
      let c = mappings[i];
      let digit = B64.iter().position(|&b| b == c).expect("invalid VLQ char") as i64;
      i += 1;
      value |= (digit & 0x1f) << shift;
      shift += 5;
      if digit & 0x20 == 0 {
        break;
      }
    }
    let is_negative = value & 1 == 1;
    value >>= 1;
    if is_negative {
      value = -value;
    }
    (value, i - start)
  }

  /// Parse a VLQ source map mappings string and return all segments.
  /// Each segment is (gen_line 0-based, gen_col 0-based, src_idx, orig_line 0-based, orig_col 0-based).
  fn parse_mappings(mappings_str: &str) -> Vec<(usize, usize, usize, usize, usize)> {
    let bytes = mappings_str.as_bytes();
    let mut result = vec![];
    let mut gen_line: usize = 0;
    let mut gen_col: i64 = 0;
    let mut src_idx: i64 = 0;
    let mut orig_line: i64 = 0;
    let mut orig_col: i64 = 0;
    let mut pos = 0;

    while pos < bytes.len() {
      if bytes[pos] == b';' {
        gen_line += 1;
        gen_col = 0;
        pos += 1;
        continue;
      }
      if bytes[pos] == b',' {
        pos += 1;
        continue;
      }
      // Decode segment: gen_col_delta[, src_delta, orig_line_delta, orig_col_delta[, name_delta]]
      let (d, consumed) = decode_vlq(bytes, pos);
      pos += consumed;
      gen_col += d;

      if pos < bytes.len() && bytes[pos] != b',' && bytes[pos] != b';' {
        let (d, consumed) = decode_vlq(bytes, pos);
        pos += consumed;
        src_idx += d;
        let (d, consumed) = decode_vlq(bytes, pos);
        pos += consumed;
        orig_line += d;
        let (d, consumed) = decode_vlq(bytes, pos);
        pos += consumed;
        orig_col += d;

        // Optional name index
        if pos < bytes.len() && bytes[pos] != b',' && bytes[pos] != b';' {
          let (_d, consumed) = decode_vlq(bytes, pos);
          pos += consumed;
        }

        result.push((
          gen_line,
          gen_col as usize,
          src_idx as usize,
          orig_line as usize,
          orig_col as usize,
        ));
      } else {
        // Segment with only gen_col (no source mapping)
        result.push((gen_line, gen_col as usize, 0, 0, 0));
      }
    }
    result
  }

  /// Find the mapping closest to (but not after) the given generated column on the given line.
  fn find_mapping(
    mappings: &[(usize, usize, usize, usize, usize)],
    gen_line: usize,
    gen_col: usize,
  ) -> Option<(usize, usize, usize, usize, usize)> {
    mappings
      .iter()
      .filter(|(gl, gc, _, _, _)| *gl == gen_line && *gc <= gen_col)
      .max_by_key(|(_, gc, _, _, _)| *gc)
      .copied()
  }

  #[test]
  fn source_map_member_access_maps_to_original_identifier() {
    // Input: scope-hoisted code where `$modId` is a require'd module namespace.
    // After inline requires: `$modId.collectAll(collection)` becomes
    // `(0, require("module-id")).collectAll(collection)`
    //
    // We want `.collectAll` in the output to map back to where `$modId.collectAll`
    // was used in the input (specifically the `.collectAll` property access).
    let code = r#"
const $modId = require("module-id");

function run(collection, events, experience) {
    return $modId.collectAll(collection)(events, experience);
}
    "#
    .trim();

    let RunVisitResult {
      output_code,
      source_map,
      ..
    } = run_test_visit(code, |ctx| InlineRequiresOptimizer {
      unresolved_mark: ctx.unresolved_mark,
      ..Default::default()
    });

    // Verify the output has the expected shape
    let output = output_code.trim();
    assert!(
      output.contains(r#"(0, require("module-id")).collectAll"#),
      "Output should contain inline require wrapper with member access"
    );

    // Find `.collectAll(` in the output
    let output_collect_all_dot = output
      .find(".collectAll(")
      .expect(".collectAll( not found in output");
    let output_collect_all_col = output_collect_all_dot + 1; // skip the '.'

    // Find `$modId.collectAll(` in the input (the usage, not the declaration).
    let input_usage = code.find("$modId.collectAll(").expect("usage not found");
    let input_dot_collect_abs = input_usage + "$modId".len();
    let input_ident_collect_abs = input_dot_collect_abs + 1; // `collectAll` starts here
    let input_before = &code[..input_usage];
    let input_line = input_before.matches('\n').count(); // 0-based
    let input_mod_col = input_usage - input_before.rfind('\n').map(|p| p + 1).unwrap_or(0);
    let input_collect_col =
      input_ident_collect_abs - input_before.rfind('\n').map(|p| p + 1).unwrap_or(0);

    // Parse the source map
    let sm_json: serde_json::Value =
      serde_json::from_slice(&source_map).expect("invalid source map JSON");
    let mappings_str = sm_json["mappings"].as_str().expect("no mappings field");
    let all_mappings = parse_mappings(mappings_str);

    // Find the mapping for .collectAll in the output
    let output_before = &output[..output_collect_all_col];
    let output_gen_line = output_before.matches('\n').count();
    let output_gen_col =
      output_collect_all_col - output_before.rfind('\n').map(|p| p + 1).unwrap_or(0);

    let mapping = find_mapping(&all_mappings, output_gen_line, output_gen_col);
    assert!(
      mapping.is_some(),
      "Should find a mapping for .collectAll in the output"
    );
    let (_gen_line, _gen_col, _src, orig_line, orig_col) = mapping.unwrap();

    // The mapping should point to the `$modId.collectAll(` usage in the input.
    // Ideally it points to either `$modId` (the object) or `.collectAll` (the property).
    // Both are acceptable since the inline require replaces `$modId` and the member
    // expression `$modId.collectAll` is preserved.
    assert_eq!(
      orig_line, input_line,
      "collectAll mapping should point to the correct line in the input"
    );

    // Accept mapping to `$modId` col OR `.collectAll` col OR `collectAll` col
    let acceptable_cols = [
      input_mod_col,
      input_mod_col + "$modId".len(),
      input_collect_col,
    ];
    assert!(
      acceptable_cols.contains(&orig_col),
      "collectAll mapping should point to $modId (col {}) or .collectAll (col {}) or collectAll (col {}), got col {}",
      input_mod_col,
      input_mod_col + "$modId".len(),
      input_collect_col,
      orig_col
    );
  }
}
