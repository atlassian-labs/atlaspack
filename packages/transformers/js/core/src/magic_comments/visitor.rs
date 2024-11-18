use std::collections::HashMap;
use std::rc::Rc;

use regex::Regex;
use swc_core::common::Span;
use swc_core::ecma::ast::*;
use swc_core::ecma::utils::ExprExt;
use swc_core::ecma::visit::{Visit, VisitWith};

thread_local! {
  static RE_CHUNK_NAME: Rc<Regex> = Rc::new(Regex::new(r#"webpackChunkName:\s*['"](?<name>[^'"]+)['"]"#).unwrap());
}

#[derive(Debug)]
pub struct MagicCommentsVisitor<'a> {
  pub magic_comments: HashMap<String, String>,
  pub offset: u32,
  pub code: &'a str,
}

impl<'a> MagicCommentsVisitor<'a> {
  pub fn new(code: &'a str) -> Self {
    Self {
      magic_comments: Default::default(),
      offset: 0,
      code,
    }
  }

  fn fix_callee(&self, span: &Span) -> (u32, u32) {
    let start = span.lo().0 - self.offset;
    let end = span.hi().0 - self.offset;
    (start, end)
  }
}

impl<'a> Visit for MagicCommentsVisitor<'a> {
  fn visit_module(&mut self, node: &Module) {
    self.offset = node.span.lo().0;
    node.visit_children_with(self);
  }

  fn visit_call_expr(&mut self, node: &CallExpr) {
    if !node.callee.is_import() || node.args.len() == 0 || !node.args[0].expr.is_str() {
      node.visit_children_with(self);
      return;
    }

    let (code_start, code_end) = self.fix_callee(&node.span);
    let code_start_index = (code_start - 1) as usize;
    let code_end_index = (code_end - 1) as usize;
    let inner = &self.code[code_start_index..code_end_index];

    let re = RE_CHUNK_NAME.with(|re| re.clone());

    let Some(caps) = re.captures(inner) else {
      return;
    };

    let Some(found) = caps.name("name") else {
      return;
    };

    let Expr::Lit(Lit::Str(specifier)) = &*node.args[0].expr else {
      return;
    };

    self
      .magic_comments
      .insert(specifier.value.to_string(), found.as_str().to_string());
  }
}
