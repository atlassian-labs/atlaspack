use serde::Deserialize;
use swc_core::atoms::Atom;
use swc_core::common::{Mark, Span, DUMMY_SP};
use swc_core::ecma::ast::{
  self, CallExpr, ExprOrSpread, Ident, Lit, MemberExpr, MemberProp, ModuleItem, Stmt, Str,
};
use swc_core::ecma::atoms::JsWord;
use swc_core::ecma::utils::private_ident;
use swc_core::ecma::visit::VisitMutWith;
use swc_core::quote;
use swc_core::{
  atoms::atom,
  ecma::{
    ast::{Callee, Expr},
    visit::VisitMut,
  },
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextualImportsConfig {
  #[serde(default)]
  pub server: bool,
  #[serde(default)]
  pub default_if_undefined: bool,
}

impl Default for ContextualImportsConfig {
  fn default() -> ContextualImportsConfig {
    ContextualImportsConfig {
      server: false,
      default_if_undefined: false,
    }
  }
}

pub struct ContextualImportsInlineRequireVisitor {
  unresolved_mark: Mark,
  config: ContextualImportsConfig,
  new_stmts: Vec<Stmt>,
}

impl ContextualImportsInlineRequireVisitor {
  pub fn new(unresolved_mark: Mark, config: ContextualImportsConfig) -> Self {
    ContextualImportsInlineRequireVisitor {
      unresolved_mark,
      config,

      new_stmts: vec![],
    }
  }

  fn create_import(&mut self, atom: Atom, span: Span) -> Expr {
    CallExpr {
      callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
        "require".into(),
        // Required so that we resolve the new dependency
        DUMMY_SP.apply_mark(self.unresolved_mark),
      )))),
      args: vec![ExprOrSpread {
        expr: Box::new(Expr::Lit(Lit::Str(Str {
          value: atom,
          // This span is important to avoid getting marked as a helper
          span: span,
          raw: None,
        }))),
        spread: None,
      }],
      span: DUMMY_SP,
      type_args: None,
    }
    .into()
  }

  fn create_conditional_expr(
    &mut self,
    cond: Atom,
    if_true: Atom,
    if_true_span: Span,
    if_false: Atom,
    if_false_span: Span,
  ) -> Expr {
    let cond: Expr = Expr::Lit(Lit::Str(cond.into()));

    let ternary_cond = if self.config.default_if_undefined {
      quote!(
        "globalThis.__MCOND && globalThis.__MCOND($cond)" as Expr,
        cond: Expr = cond,
      )
    } else {
      quote!("globalThis.__MCOND($cond)" as Expr, cond: Expr = cond)
    };

    quote!(
      "$ternary_cond ? $if_true.default : $if_false.default" as Expr,
      ternary_cond: Expr = ternary_cond,
      if_true: Expr = self.create_import(if_true, if_true_span),
      if_false: Expr = self.create_import(if_false, if_false_span)
    )
  }

  fn create_lazy_server_object(
    &mut self,
    cond: Atom,
    if_true: Atom,
    if_true_span: Span,
    if_false: Atom,
    if_false_span: Span,
  ) -> Expr {
    let obj_ident = private_ident!(format!(
      "{}{}{}",
      cond.as_str(),
      if_true.as_str(),
      if_false.as_str()
    ));

    let if_true_expr = self.create_import(if_true, if_true_span);
    let if_false_expr = self.create_import(if_false, if_false_span);

    self.new_stmts.push(quote!(
      "const $obj_ident = {
        ifTrue: $if_true.default,
        ifFalse: $if_false.default
      };" as Stmt,
      obj_ident: Ident = obj_ident.clone(),
      if_true: Expr = if_true_expr,
      if_false: Expr = if_false_expr
    ));

    self.new_stmts.push(quote!(
      r#"
      Object.defineProperty($obj_ident, "load", {
        get: () => globalThis.__MCOND && globalThis.__MCOND($cond) ? $obj_ident.ifTrue : $obj_ident.ifFalse
      });
      "# as Stmt,
      cond: Expr = Expr::Lit(Lit::Str(cond.into())),
      obj_ident: Ident = obj_ident.clone()
    ));

    MemberExpr {
      span: DUMMY_SP,
      obj: obj_ident.into(),
      prop: MemberProp::Ident(Ident::new("load".into(), DUMMY_SP)),
    }
    .into()
  }

  fn match_str(node: &ast::Expr) -> Option<(JsWord, Span)> {
    use ast::*;

    match node {
      // "string" or 'string'
      Expr::Lit(Lit::Str(s)) => Some((s.value.clone(), s.span)),
      // `string`
      Expr::Tpl(tpl) if tpl.quasis.len() == 1 && tpl.exprs.is_empty() => {
        Some(((*tpl.quasis[0].raw).into(), tpl.span))
      }
      _ => None,
    }
  }
}

impl VisitMut for ContextualImportsInlineRequireVisitor {
  fn visit_mut_expr(&mut self, node: &mut Expr) {
    let Expr::Call(call) = node else {
      return;
    };

    let Callee::Expr(callee_expr) = &call.callee else {
      return;
    };

    let Expr::Ident(callee_ident) = &**callee_expr else {
      return;
    };

    if callee_ident.sym != atom!("importCond") && call.args.len() != 3 {
      // Not an importCond
      return;
    }

    if callee_ident.span.ctxt.outer() != self.unresolved_mark {
      // Don't process if the `importCond` identifier is shadowed
      return;
    }

    let (Some((cond, _cond_span)), Some((if_true, if_true_span)), Some((if_false, if_false_span))) = (
      ContextualImportsInlineRequireVisitor::match_str(&call.args[0].expr),
      ContextualImportsInlineRequireVisitor::match_str(&call.args[1].expr),
      ContextualImportsInlineRequireVisitor::match_str(&call.args[2].expr),
    ) else {
      return;
    };

    match self.config.server {
      true => {
        // Write special expression for server
        *node = self.create_lazy_server_object(cond, if_true, if_true_span, if_false, if_false_span)
      }
      false => {
        // Update inline expression
        *node = self.create_conditional_expr(cond, if_true, if_true_span, if_false, if_false_span);
      }
    };
  }

  fn visit_mut_module_items(&mut self, stmts: &mut Vec<ModuleItem>) {
    stmts.visit_mut_children_with(self);

    let mut stmts_updated: Vec<ModuleItem> = Vec::with_capacity(stmts.len() + self.new_stmts.len());

    stmts_updated.append(
      &mut self
        .new_stmts
        .iter_mut()
        .map(|stmt| ModuleItem::Stmt(stmt.clone()))
        .collect(),
    );

    stmts_updated.append(stmts);

    *stmts = stmts_updated;
  }
}

#[cfg(test)]
mod tests {
  use atlaspack_swc_runner::{
    runner::RunVisitResult,
    test_utils::{remove_code_whitespace, run_test_visit},
  };

  use crate::{ContextualImportsConfig, ContextualImportsInlineRequireVisitor};

  #[test]
  fn test_import_cond() {
    let input_code = r#"
      const x = importCond('condition-1', 'a', 'b');
      const y = importCond('condition-2', 'c', 'd');
      const z = importCond('condition-2', 'c', 'd');
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      ContextualImportsInlineRequireVisitor::new(
        context.unresolved_mark,
        ContextualImportsConfig::default(),
      )
    });

    let expected_code = r#"
      const x = globalThis.__MCOND("condition-1") ? require("a").default : require("b").default;
      const y = globalThis.__MCOND("condition-2") ? require("c").default : require("d").default;
      const z = globalThis.__MCOND("condition-2") ? require("c").default : require("d").default;
    "#;

    assert_eq!(
      remove_code_whitespace(output_code.as_str()),
      remove_code_whitespace(expected_code)
    );
  }

  #[test]
  fn test_import_cond_with_default() {
    let input_code = r#"
      const x = importCond('condition-1', 'a', 'b');
      const y = importCond('condition-2', 'c', 'd');
      const z = importCond('condition-2', 'c', 'd');
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      ContextualImportsInlineRequireVisitor::new(
        context.unresolved_mark,
        ContextualImportsConfig {
          default_if_undefined: true,
          ..Default::default()
        },
      )
    });

    let expected_code = r#"
      const x = globalThis.__MCOND && globalThis.__MCOND("condition-1") ? require("a").default : require("b").default;
      const y = globalThis.__MCOND && globalThis.__MCOND("condition-2") ? require("c").default : require("d").default;
      const z = globalThis.__MCOND && globalThis.__MCOND("condition-2") ? require("c").default : require("d").default;
    "#;

    assert_eq!(
      remove_code_whitespace(output_code.as_str()),
      remove_code_whitespace(expected_code)
    );
  }

  #[test]
  fn test_import_cond_shadowed_variable() {
    let input_code = r#"
      importCond('condition', 'a', 'b');

      function some_function(importCond) {
        importCond('condition', 'a', 'b');
      }
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      ContextualImportsInlineRequireVisitor::new(
        context.unresolved_mark,
        ContextualImportsConfig::default(),
      )
    });

    let expected_code = r#"
      globalThis.__MCOND("condition") ? require("a").default : require("b").default;

      function some_function(importCond) {
        importCond('condition', 'a', 'b');
      }
    "#;

    assert_eq!(
      remove_code_whitespace(output_code.as_str()),
      remove_code_whitespace(expected_code)
    );
  }

  #[test]
  fn test_import_cond_server() {
    let input_code = r#"
      importCond('condition', 'a', 'b');

      console.log('After condition');
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      ContextualImportsInlineRequireVisitor::new(
        context.unresolved_mark,
        ContextualImportsConfig {
          server: true,
          ..Default::default()
        },
      )
    });

    let expected_code = r#"
      const conditionab = {
        ifTrue: require("a").default,
        ifFalse: require("b").default
      };
      Object.defineProperty(conditionab, "load", {
        get: ()=>globalThis.__MCOND && globalThis.__MCOND("condition") ? conditionab.ifTrue : conditionab.ifFalse
      });
      conditionab.load;

      console.log('After condition');
    "#;

    assert_eq!(
      remove_code_whitespace(output_code.as_str()),
      remove_code_whitespace(expected_code)
    );
  }

  #[test]
  fn test_import_cond_server_shadowed_variable() {
    let input_code = r#"
      importCond('condition', 'a', 'b');

      function some_function(importCond) {
        importCond('condition', 'a', 'b');
      }
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      ContextualImportsInlineRequireVisitor::new(
        context.unresolved_mark,
        ContextualImportsConfig {
          server: true,
          ..Default::default()
        },
      )
    });

    let expected_code = r#"
      const conditionab = {
        ifTrue: require("a").default,
        ifFalse: require("b").default
      };
      Object.defineProperty(conditionab, "load", {
        get: ()=>globalThis.__MCOND && globalThis.__MCOND("condition") ? conditionab.ifTrue : conditionab.ifFalse
      });
      conditionab.load;

      function some_function(importCond) {
        importCond('condition', 'a', 'b');
      }
    "#;

    assert_eq!(
      remove_code_whitespace(output_code.as_str()),
      remove_code_whitespace(expected_code)
    );
  }
}
