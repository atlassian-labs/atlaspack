use swc_core::ecma::ast::{Callee, Expr};

use crate::types::TransformState;

/// Returns `true` when the provided expression is a `jsx` call emitted by the
/// Babel JSX transform that should not appear once Compiled has run.
pub fn is_transformed_jsx_function(expr: &Expr, state: &TransformState) -> bool {
  if state.compiled_imports.is_none() {
    return false;
  }

  let Expr::Call(call) = expr else {
    return false;
  };

  let Callee::Expr(callee) = &call.callee else {
    return false;
  };

  let Expr::Ident(ident) = &**callee else {
    return false;
  };

  let name = ident.sym.as_ref();

  if name == "jsx" {
    return true;
  }

  if let Some(local) = &state.pragma.classic_jsx_pragma_local_name {
    if name == local {
      return true;
    }
  }

  false
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::{PluginOptions, TransformFile, TransformState};
  use swc_core::common::{DUMMY_SP, SyntaxContext};
  use swc_core::ecma::ast::{CallExpr, Expr, ExprOrSpread, Ident, Lit, Str};

  fn build_state() -> TransformState {
    TransformState::new(TransformFile::default(), PluginOptions::default())
  }

  fn call(name: &str) -> Expr {
    Expr::Call(CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
        name.into(),
        DUMMY_SP,
        SyntaxContext::empty(),
      )))),
      args: vec![ExprOrSpread {
        spread: None,
        expr: Box::new(Expr::Lit(Lit::Str(Str::from("value")))),
      }],
      type_args: None,
    })
  }

  fn enable_compiled(mut state: TransformState) -> TransformState {
    if let Some(mut imports) = state.compiled_imports.take() {
      imports.css.push("css".into());
      state.compiled_imports = Some(imports);
    } else {
      let mut imports = crate::types::CompiledImports::default();
      imports.css.push("css".into());
      state.compiled_imports = Some(imports);
    }
    state
  }

  #[test]
  fn returns_false_when_no_compiled_imports_present() {
    let state = build_state();
    let expr = call("jsx");

    assert!(!is_transformed_jsx_function(&expr, &state));
  }

  #[test]
  fn returns_true_for_jsx_identifier() {
    let state = enable_compiled(build_state());
    let expr = call("jsx");

    assert!(is_transformed_jsx_function(&expr, &state));
  }

  #[test]
  fn returns_true_for_classic_jsx_local_name() {
    let mut state = enable_compiled(build_state());
    state.pragma.classic_jsx_pragma_local_name = Some("_jsx".into());

    let expr = call("_jsx");

    assert!(is_transformed_jsx_function(&expr, &state));
  }

  #[test]
  fn returns_false_for_non_jsx_calls() {
    let state = enable_compiled(build_state());
    let expr = call("css");

    assert!(!is_transformed_jsx_function(&expr, &state));
  }
}
