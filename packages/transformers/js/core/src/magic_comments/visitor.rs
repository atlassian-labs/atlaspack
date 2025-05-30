use std::collections::HashMap;

use regex::Regex;
use std::sync::LazyLock;
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::Visit;
use swc_core::ecma::visit::VisitWith;

static RE_CHUNK_NAME: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r#"webpackChunkName:\s*['"](?<name>[^'"]+)['"]"#).unwrap());
const MAGIC_COMMENT_DEFAULT_KEYWORD: &str = "webpackChunkName";

/// MagicCommentsVisitor will scan code for Webpack Magic Comments
#[derive(Debug)]
pub struct MagicCommentsVisitor<'a> {
  pub magic_comments: HashMap<String, String>,
  pub code: &'a str,
}

impl<'a> MagicCommentsVisitor<'a> {
  pub fn new(code: &'a str) -> Self {
    Self {
      magic_comments: Default::default(),
      code,
    }
  }

  pub fn has_magic_comment(code: &str) -> bool {
    code.contains(MAGIC_COMMENT_DEFAULT_KEYWORD)
  }
}

impl Visit for MagicCommentsVisitor<'_> {
  fn visit_call_expr(&mut self, node: &CallExpr) {
    if !node.callee.is_import() {
      node.visit_children_with(self);
      return;
    }

    let Some(expr) = node.args.first() else {
      return;
    };

    let Expr::Lit(Lit::Str(specifier)) = &*expr.expr else {
      return;
    };

    // Comments are not available in the AST so we have to get the start/end
    // positions of the code for the call expression and match it with a regular
    // expression that matches the magic comment keyword within the code slice

    // swc index starts at 1
    let code_start_index = (node.span.lo().0 - 1) as usize;
    let code_end_index = (node.span.hi().0 - 1) as usize;
    let slice = &self.code[code_start_index..code_end_index];

    let Some(found) = match_re(slice) else {
      return;
    };

    self
      .magic_comments
      .insert(specifier.value.to_string(), found.as_str().to_string());
  }
}

fn match_re(src: &str) -> Option<String> {
  RE_CHUNK_NAME
    .captures(src)
    .and_then(|caps| caps.name("name").map(|found| found.as_str().to_string()))
}
