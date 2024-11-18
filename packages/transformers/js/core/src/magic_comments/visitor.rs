use std::collections::HashMap;

use regex::Regex;
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::Visit;
use swc_core::ecma::visit::VisitWith;

thread_local! {
  static RE_CHUNK_NAME: Regex = Regex::new(r#"webpackChunkName:\s*['"](?<name>[^'"]+)['"]"#).unwrap();
}

const MAGIC_COMMENT_DEFAULT_KEYWORD: &str = "webpackChunkName";

/// MagicCommentsVisitor will scan code for Webpack Magic Comments
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

  pub fn has_magic_comment(code: &str) -> bool {
    code.contains(MAGIC_COMMENT_DEFAULT_KEYWORD)
  }
}

impl<'a> Visit for MagicCommentsVisitor<'a> {
  fn visit_module(&mut self, node: &Module) {
    self.offset = node.span.lo().0;
    node.visit_children_with(self);
  }

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
    let code_start = (node.span.lo().0 - self.offset) as usize;
    let code_end = (node.span.hi().0 - self.offset) as usize;

    // swc index starts at 1
    let code_start_index = code_start - 1;
    let code_end_index = code_end - 1;

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
  RE_CHUNK_NAME.with(|re| {
    let caps = re.captures(src)?;
    let found = caps.name("name")?;
    Some(found.as_str().to_string())
  })
}
