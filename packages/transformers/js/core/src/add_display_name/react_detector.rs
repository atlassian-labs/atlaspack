use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Visit, VisitWith};

/// Visitor that sets `is_react` to `true` once JSX or a hook call is encountered.
#[derive(Default)]
pub struct ReactDetector {
  is_react: bool,
}

impl ReactDetector {
  pub fn is_react(&self) -> bool {
    self.is_react
  }
}

impl Visit for ReactDetector {
  fn visit_jsx_element(&mut self, _: &JSXElement) {
    self.is_react = true;
  }

  fn visit_jsx_fragment(&mut self, _: &JSXFragment) {
    self.is_react = true;
  }

  fn visit_call_expr(&mut self, n: &CallExpr) {
    if let Callee::Expr(expr) = &n.callee {
      if let Expr::Ident(id) = &**expr {
        let sym = id.sym.as_ref();
        if sym.starts_with("use")
          && sym.len() > 3
          && sym[3..].chars().next().map_or(false, |c| c.is_uppercase())
        {
          self.is_react = true;
        }
      }
    }
    if !self.is_react {
      n.visit_children_with(self);
    }
  }
}

pub fn is_component_name(id: &Ident) -> bool {
  id.sym
    .chars()
    .next()
    .map(|c| c.is_uppercase())
    .unwrap_or(false)
}

pub fn function_contains_component(func: &Function) -> bool {
  func
    .body
    .as_ref()
    .map(|body| {
      let mut v = ReactDetector::default();
      body.visit_with(&mut v);
      v.is_react()
    })
    .unwrap_or(false)
}

pub fn arrow_contains_component(arrow: &ArrowExpr) -> bool {
  match &*arrow.body {
    BlockStmtOrExpr::BlockStmt(block) => {
      let mut v = ReactDetector::default();
      block.visit_with(&mut v);
      v.is_react()
    }
    BlockStmtOrExpr::Expr(expr) => {
      let mut v = ReactDetector::default();
      expr.visit_with(&mut v);
      v.is_react()
    }
  }
}

#[cfg(test)]
mod tests {
  use atlaspack_swc_runner::runner::{
    RunWithTransformationOptions, RunWithTransformationOutput, run_with_transformation,
  };
  use swc_core::{
    common::{DUMMY_SP, SyntaxContext},
    ecma::visit::VisitWith,
  };
  use swc_ecma_parser::{EsSyntax, Syntax};

  use super::*;

  fn run_is_react(code: &str) -> bool {
    let options = RunWithTransformationOptions {
      code,
      syntax: Some(Syntax::Es(EsSyntax {
        jsx: true,
        ..Default::default()
      })),
    };

    let RunWithTransformationOutput {
      transform_result: visitor,
      ..
    } = run_with_transformation(options, |_ctx, module| {
      let mut visitor = ReactDetector::default();
      module.visit_with(&mut visitor);
      visitor
    })
    .unwrap();

    visitor.is_react()
  }

  #[test]
  fn test_react_detector() {
    let code = "export function Foo() { return <div />; }";
    assert!(run_is_react(code));
  }

  #[test]
  fn test_react_detector_with_jsx_fragment() {
    let code = "export function Foo() { return <></>; }";
    assert!(run_is_react(code));
  }

  #[test]
  fn test_react_detector_with_hook() {
    let code = "export function Foo() { useEffect(() => {}, []); return null; }";
    assert!(run_is_react(code));
  }

  #[test]
  fn test_react_detector_without_jsx() {
    let code = "export function Foo() { return 1; }";
    assert!(!run_is_react(code));
  }

  #[test]
  fn test_is_component_name_cases() {
    let upper = Ident::new("Foo".into(), DUMMY_SP, SyntaxContext::empty());
    let lower = Ident::new("bar".into(), DUMMY_SP, SyntaxContext::empty());
    assert!(is_component_name(&upper));
    assert!(!is_component_name(&lower));
  }
}
