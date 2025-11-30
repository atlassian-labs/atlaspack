use swc_core::ecma::ast::{Callee, Expr, Ident, MemberExpr, TaggedTpl};

use crate::types::TransformState;

fn compiled_import_matches(list: &[String], name: &str) -> bool {
  list.iter().any(|item| item == name)
}

fn is_identifier_callee<'a>(callee: &'a Callee) -> Option<&'a Ident> {
  match callee {
    Callee::Expr(expr) => match &**expr {
      Expr::Ident(ident) => Some(ident),
      _ => None,
    },
    Callee::Super(_) | Callee::Import(_) => None,
  }
}

fn is_identifier_expr(expr: &Expr) -> Option<&Ident> {
  match expr {
    Expr::Ident(ident) => Some(ident),
    _ => None,
  }
}

fn is_member_expr(expr: &Expr) -> Option<&MemberExpr> {
  match expr {
    Expr::Member(member) => Some(member),
    _ => None,
  }
}

fn is_compiled_styled_member_expression(expr: &Expr, state: &TransformState) -> bool {
  let member = match is_member_expr(expr) {
    Some(member) => member,
    None => return false,
  };

  let Some(ident) = is_identifier_expr(member.obj.as_ref()) else {
    return false;
  };

  state
    .compiled_imports
    .as_ref()
    .map(|imports| compiled_import_matches(&imports.styled, ident.sym.as_ref()))
    .unwrap_or(false)
}

fn is_compiled_styled_composition_call_expression(expr: &Expr, state: &TransformState) -> bool {
  let Expr::Call(call) = expr else {
    return false;
  };

  let Some(ident) = is_identifier_callee(&call.callee) else {
    return false;
  };

  state
    .compiled_imports
    .as_ref()
    .map(|imports| compiled_import_matches(&imports.styled, ident.sym.as_ref()))
    .unwrap_or(false)
}

fn is_compiled_css_identifier(name: &str, state: &TransformState) -> bool {
  if state
    .compiled_imports
    .as_ref()
    .map(|imports| compiled_import_matches(&imports.css, name))
    .unwrap_or(false)
  {
    return true;
  }

  if let Some(imported) = state.imported_compiled_imports.css.as_ref() {
    if imported == name {
      return true;
    }
  }

  false
}

fn is_compiled_keyframes_identifier(name: &str, state: &TransformState) -> bool {
  state
    .compiled_imports
    .as_ref()
    .map(|imports| compiled_import_matches(&imports.keyframes, name))
    .unwrap_or(false)
}

fn is_compiled_css_map_identifier(name: &str, state: &TransformState) -> bool {
  state
    .compiled_imports
    .as_ref()
    .map(|imports| compiled_import_matches(&imports.css_map, name))
    .unwrap_or(false)
}

/// Returns `true` if the expression represents a `css` call sourced from
/// `@compiled/react`.
pub fn is_compiled_css_call_expression(expr: &Expr, state: &TransformState) -> bool {
  let Expr::Call(call) = expr else {
    return false;
  };

  let Some(ident) = is_identifier_callee(&call.callee) else {
    return false;
  };

  is_compiled_css_identifier(ident.sym.as_ref(), state)
}

/// Returns `true` if the expression represents a `css` tagged template sourced
/// from `@compiled/react`.
pub fn is_compiled_css_tagged_template_expression(expr: &Expr, state: &TransformState) -> bool {
  let Expr::TaggedTpl(tagged) = expr else {
    return false;
  };

  let Some(ident) = is_identifier_expr(&tagged.tag) else {
    return false;
  };

  state
    .compiled_imports
    .as_ref()
    .map(|imports| compiled_import_matches(&imports.css, ident.sym.as_ref()))
    .unwrap_or(false)
}

/// Returns `true` if the expression represents a `keyframes` call sourced from
/// `@compiled/react`.
pub fn is_compiled_keyframes_call_expression(expr: &Expr, state: &TransformState) -> bool {
  let Expr::Call(call) = expr else {
    return false;
  };

  let Some(ident) = is_identifier_callee(&call.callee) else {
    return false;
  };

  is_compiled_keyframes_identifier(ident.sym.as_ref(), state)
}

/// Returns `true` if the expression represents a `cssMap` call sourced from
/// `@compiled/react`.
pub fn is_compiled_css_map_call_expression(expr: &Expr, state: &TransformState) -> bool {
  let Expr::Call(call) = expr else {
    return false;
  };

  let Some(ident) = is_identifier_callee(&call.callee) else {
    return false;
  };

  is_compiled_css_map_identifier(ident.sym.as_ref(), state)
}

/// Returns `true` if the expression represents a `keyframes` tagged template
/// sourced from `@compiled/react`.
pub fn is_compiled_keyframes_tagged_template_expression(
  expr: &Expr,
  state: &TransformState,
) -> bool {
  let Expr::TaggedTpl(tagged) = expr else {
    return false;
  };

  let Some(ident) = is_identifier_expr(&tagged.tag) else {
    return false;
  };

  is_compiled_keyframes_identifier(ident.sym.as_ref(), state)
}

/// Returns `true` if the expression represents a `styled` call sourced from
/// `@compiled/react`.
pub fn is_compiled_styled_call_expression(expr: &Expr, state: &TransformState) -> bool {
  let Expr::Call(call) = expr else {
    return false;
  };

  let Callee::Expr(callee) = &call.callee else {
    return false;
  };

  if is_compiled_styled_member_expression(callee, state) {
    return true;
  }

  is_compiled_styled_composition_call_expression(callee, state)
}

/// Returns `true` if the expression represents a `styled` tagged template
/// sourced from `@compiled/react`.
pub fn is_compiled_styled_tagged_template_expression(expr: &Expr, state: &TransformState) -> bool {
  let Expr::TaggedTpl(TaggedTpl { tag, .. }) = expr else {
    return false;
  };

  if is_compiled_styled_member_expression(tag, state) {
    return true;
  }

  is_compiled_styled_composition_call_expression(tag, state)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::{CompiledImports, PluginOptions, TransformFile, TransformState};
  use swc_core::common::{SyntaxContext, DUMMY_SP};
  use swc_core::ecma::ast::{
    CallExpr, Expr, ExprOrSpread, Ident, IdentName, Lit, MemberExpr, MemberProp, Str, TaggedTpl,
    Tpl, TplElement,
  };
  use swc_core::ecma::atoms::Atom;

  fn build_state() -> TransformState {
    let file = TransformFile::default();
    TransformState::new(file, PluginOptions::default())
  }

  fn ident(name: &str) -> Ident {
    Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty())
  }

  fn empty_tpl() -> Box<Tpl> {
    Box::new(Tpl {
      span: DUMMY_SP,
      exprs: vec![],
      quasis: vec![TplElement {
        span: DUMMY_SP,
        tail: true,
        cooked: None,
        raw: Atom::from(""),
      }],
    })
  }

  fn call_expr(name: &str) -> Expr {
    Expr::Call(CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Ident(ident(name)))),
      args: vec![],
      type_args: None,
    })
  }

  fn tagged_expr(name: &str) -> Expr {
    Expr::TaggedTpl(TaggedTpl {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      tag: Box::new(Expr::Ident(ident(name))),
      type_params: None,
      tpl: empty_tpl(),
    })
  }

  fn styled_member_call(object: &str, property: &str) -> Expr {
    Expr::Call(CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Ident(ident(object))),
        prop: MemberProp::Ident(IdentName::from(property)),
      }))),
      args: vec![],
      type_args: None,
    })
  }

  fn styled_composition_call(callee: &str) -> Expr {
    Expr::Call(CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Call(CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Callee::Expr(Box::new(Expr::Ident(ident(callee)))),
        args: vec![ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Lit(Lit::Str(Str::from("component")))),
        }],
        type_args: None,
      }))),
      args: vec![],
      type_args: None,
    })
  }

  fn styled_tagged_member(object: &str, property: &str) -> Expr {
    Expr::TaggedTpl(TaggedTpl {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      tag: Box::new(Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Ident(ident(object))),
        prop: MemberProp::Ident(IdentName::from(property)),
      })),
      type_params: None,
      tpl: empty_tpl(),
    })
  }

  fn styled_tagged_composition(callee: &str) -> Expr {
    Expr::TaggedTpl(TaggedTpl {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      tag: Box::new(Expr::Call(CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Callee::Expr(Box::new(Expr::Ident(ident(callee)))),
        args: vec![ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Ident(ident("Component"))),
        }],
        type_args: None,
      })),
      type_params: None,
      tpl: empty_tpl(),
    })
  }

  fn with_compiled_imports<F>(mut state: TransformState, update: F) -> TransformState
  where
    F: FnOnce(&mut CompiledImports),
  {
    let mut imports = state.compiled_imports.take().unwrap_or_default();
    update(&mut imports);
    state.compiled_imports = Some(imports);
    state
  }

  #[test]
  fn detects_compiled_css_call_expression() {
    let state = with_compiled_imports(build_state(), |imports| {
      imports.css.push("css".into());
    });

    let expr = call_expr("css");

    assert!(is_compiled_css_call_expression(&expr, &state));
  }

  #[test]
  fn detects_compiled_css_call_expression_via_runtime_import() {
    let mut state = build_state();
    state.imported_compiled_imports.css = Some("_css".into());

    let expr = call_expr("_css");

    assert!(is_compiled_css_call_expression(&expr, &state));
  }

  #[test]
  fn rejects_non_compiled_css_call_expression() {
    let state = build_state();
    let expr = call_expr("emotionCss");

    assert!(!is_compiled_css_call_expression(&expr, &state));
  }

  #[test]
  fn detects_compiled_css_tagged_template_expression() {
    let state = with_compiled_imports(build_state(), |imports| {
      imports.css.push("css".into());
    });

    let expr = tagged_expr("css");

    assert!(is_compiled_css_tagged_template_expression(&expr, &state));
  }

  #[test]
  fn detects_compiled_keyframes_call_expression() {
    let state = with_compiled_imports(build_state(), |imports| {
      imports.keyframes.push("keyframes".into());
    });

    let expr = call_expr("keyframes");

    assert!(is_compiled_keyframes_call_expression(&expr, &state));
  }

  #[test]
  fn detects_compiled_css_map_call_expression() {
    let state = with_compiled_imports(build_state(), |imports| {
      imports.css_map.push("cssMap".into());
    });

    let expr = call_expr("cssMap");

    assert!(is_compiled_css_map_call_expression(&expr, &state));
  }

  #[test]
  fn detects_compiled_keyframes_tagged_template_expression() {
    let state = with_compiled_imports(build_state(), |imports| {
      imports.keyframes.push("keyframes".into());
    });

    let expr = tagged_expr("keyframes");

    assert!(is_compiled_keyframes_tagged_template_expression(
      &expr, &state
    ));
  }

  #[test]
  fn detects_compiled_styled_call_expression_from_member() {
    let state = with_compiled_imports(build_state(), |imports| {
      imports.styled.push("styled".into());
    });

    let expr = styled_member_call("styled", "div");

    assert!(is_compiled_styled_call_expression(&expr, &state));
  }

  #[test]
  fn detects_compiled_styled_call_expression_from_composition() {
    let state = with_compiled_imports(build_state(), |imports| {
      imports.styled.push("styled".into());
    });

    let expr = styled_composition_call("styled");

    assert!(is_compiled_styled_call_expression(&expr, &state));
  }

  #[test]
  fn detects_compiled_styled_tagged_template_from_member() {
    let state = with_compiled_imports(build_state(), |imports| {
      imports.styled.push("styled".into());
    });

    let expr = styled_tagged_member("styled", "div");

    assert!(is_compiled_styled_tagged_template_expression(&expr, &state));
  }

  #[test]
  fn detects_compiled_styled_tagged_template_from_composition() {
    let state = with_compiled_imports(build_state(), |imports| {
      imports.styled.push("styled".into());
    });

    let expr = styled_tagged_composition("styled");

    assert!(is_compiled_styled_tagged_template_expression(&expr, &state));
  }

  #[test]
  fn rejects_non_compiled_styled_usage() {
    let state = build_state();
    let expr = styled_member_call("emotion", "div");

    assert!(!is_compiled_styled_call_expression(&expr, &state));
    assert!(!is_compiled_styled_tagged_template_expression(
      &expr, &state
    ));
  }
}
