use swc_core::ecma::ast::{
  ArrayLit, CallExpr, Callee, Expr, Ident, JSXElement, JSXElementChild, JSXElementName, JSXExpr,
  JSXExprContainer, Prop, PropOrSpread,
};

use crate::utils_is_automatic_runtime::is_automatic_runtime;
use crate::utils_is_create_element::is_create_element;

/// Invokes `handle` for every identifier pointing at an extracted style rule
/// found within the provided expression.
pub fn remove_style_declarations_from_expr<F>(expr: &Expr, handle: &mut F)
where
  F: FnMut(&Ident),
{
  match expr {
    Expr::Ident(ident) => {
      handle(ident);
    }
    Expr::Call(call) => {
      if matches_create_element(call) {
        process_create_element(call, handle);
        return;
      }

      if is_automatic_runtime(call, "jsx") {
        process_jsx_runtime(call, handle);
        return;
      }
    }
    Expr::Array(array) => {
      process_array(array, handle);
      return;
    }
    _ => {}
  }
}

/// Invokes `handle` for every identifier referenced within a `<CS>` JSX node.
pub fn remove_style_declarations_from_jsx_element<F>(element: &JSXElement, handle: &mut F)
where
  F: FnMut(&Ident),
{
  if !matches!(
      &element.opening.name,
      JSXElementName::Ident(name) if name.sym.as_ref() == "CS"
  ) {
    return;
  }

  let Some(JSXElementChild::JSXExprContainer(JSXExprContainer { expr, .. })) =
    element.children.first()
  else {
    return;
  };

  let JSXExpr::Expr(styles_expr) = expr else {
    return;
  };

  if let Expr::Array(array) = &**styles_expr {
    process_array(array, handle);
  }
}

fn matches_create_element(call: &CallExpr) -> bool {
  match &call.callee {
    Callee::Expr(expr) => is_create_element(expr),
    Callee::Super(_) | Callee::Import(_) => false,
  }
}

fn process_create_element<F>(call: &CallExpr, handle: &mut F)
where
  F: FnMut(&Ident),
{
  let styles_arg = call.args.get(2).map(|arg| &*arg.expr);
  if let Some(Expr::Array(array)) = styles_arg {
    process_array(array, handle);
  }
}

fn process_jsx_runtime<F>(call: &CallExpr, handle: &mut F)
where
  F: FnMut(&Ident),
{
  let mut children = collect_jsx_runtime_children(call);
  let Some(first_child) = children.next() else {
    return;
  };

  if let Expr::Array(array) = first_child {
    process_array(array, handle);
  }
}

fn collect_jsx_runtime_children<'a>(call: &'a CallExpr) -> impl Iterator<Item = &'a Expr> {
  call
    .args
    .get(1)
    .and_then(|arg| match &*arg.expr {
      Expr::Object(object) => Some(object),
      _ => None,
    })
    .into_iter()
    .flat_map(|object| object.props.iter())
    .filter_map(|prop| match prop {
      PropOrSpread::Prop(prop) => match &**prop {
        Prop::KeyValue(kv) => Some(&*kv.value),
        _ => None,
      },
      PropOrSpread::Spread(_) => None,
    })
}

fn process_array<F>(array: &ArrayLit, handle: &mut F)
where
  F: FnMut(&Ident),
{
  for element in &array.elems {
    let Some(expr_or_spread) = element else {
      continue;
    };
    if expr_or_spread.spread.is_some() {
      continue;
    }

    if let Expr::Ident(ident) = &*expr_or_spread.expr {
      handle(ident);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{FileName, SourceMap, SyntaxContext, DUMMY_SP};
  use swc_core::ecma::ast::{
    ArrayLit, Decl, ExprOrSpread, Ident, JSXElement, JSXElementChild, JSXElementName, JSXExpr,
    JSXExprContainer, JSXOpeningElement, Module, ModuleItem, Stmt,
  };
  use swc_ecma_parser::lexer::Lexer;
  use swc_ecma_parser::{Parser, StringInput, Syntax};

  fn parse_module(source: &str, syntax: Syntax) -> Module {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Anon.into(), source.into());
    let lexer = Lexer::new(syntax, Default::default(), StringInput::from(&*fm), None);
    let mut parser = Parser::new_from(lexer);
    parser.parse_module().expect("failed to parse module")
  }

  fn parse_expr(source: &str, syntax: Syntax) -> Expr {
    let code = format!("const result = {};", source);
    let module = parse_module(&code, syntax);
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = &module.body[0] else {
      panic!("expected variable declaration");
    };
    let Some(init) = &var.decls[0].init else {
      panic!("expected initializer expression");
    };
    (**init).clone()
  }

  #[test]
  fn collects_create_element_identifiers() {
    let expr = parse_expr(
      "React.createElement(CC, null, [_a, _b])",
      Syntax::Es(Default::default()),
    );

    let mut seen: Vec<String> = Vec::new();
    remove_style_declarations_from_expr(&expr, &mut |ident| {
      seen.push(ident.sym.to_string());
    });

    assert_eq!(seen, vec!["_a", "_b"]);
  }

  #[test]
  fn collects_jsx_runtime_identifiers() {
    let expr = parse_expr(
      "_jsx(CC, { children: [_a, React.createElement('div')] })",
      Syntax::Es(Default::default()),
    );

    let mut seen: Vec<String> = Vec::new();
    remove_style_declarations_from_expr(&expr, &mut |ident| {
      seen.push(ident.sym.to_string());
    });

    assert_eq!(seen, vec!["_a"]);
  }

  #[test]
  fn collects_jsx_element_identifiers() {
    let element = JSXElement {
      span: DUMMY_SP,
      opening: JSXOpeningElement {
        span: DUMMY_SP,
        name: JSXElementName::Ident(Ident::new("CS".into(), DUMMY_SP, SyntaxContext::empty())),
        attrs: Vec::new(),
        self_closing: false,
        type_args: None,
      },
      closing: Some(swc_core::ecma::ast::JSXClosingElement {
        span: DUMMY_SP,
        name: JSXElementName::Ident(Ident::new("CS".into(), DUMMY_SP, SyntaxContext::empty())),
      }),
      children: vec![JSXElementChild::JSXExprContainer(JSXExprContainer {
        span: DUMMY_SP,
        expr: JSXExpr::Expr(Box::new(Expr::Array(ArrayLit {
          span: DUMMY_SP,
          elems: vec![
            Some(ExprOrSpread {
              spread: None,
              expr: Box::new(Expr::Ident(Ident::new(
                "_a".into(),
                DUMMY_SP,
                SyntaxContext::empty(),
              ))),
            }),
            Some(ExprOrSpread {
              spread: None,
              expr: Box::new(Expr::Ident(Ident::new(
                "_b".into(),
                DUMMY_SP,
                SyntaxContext::empty(),
              ))),
            }),
          ],
        }))),
      })],
    };

    let mut seen: Vec<String> = Vec::new();
    remove_style_declarations_from_jsx_element(&element, &mut |ident| {
      seen.push(ident.sym.to_string());
    });

    assert_eq!(seen, vec!["_a", "_b"]);
  }

  #[test]
  fn collects_identifier_expression() {
    let expr = Expr::Ident(Ident::new("_a".into(), DUMMY_SP, SyntaxContext::empty()));

    let mut seen: Vec<String> = Vec::new();
    remove_style_declarations_from_expr(&expr, &mut |ident| {
      seen.push(ident.sym.to_string());
    });

    assert_eq!(seen, vec!["_a"]);
  }

  #[test]
  fn ignores_non_style_arguments() {
    let expr = parse_expr(
      "React.createElement('div', null, ['_a'])",
      Syntax::Es(Default::default()),
    );

    let mut seen: Vec<String> = Vec::new();
    remove_style_declarations_from_expr(&expr, &mut |ident| {
      seen.push(ident.sym.to_string());
    });

    assert!(seen.is_empty());
  }
}
