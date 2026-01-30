use swc_core::common::{DUMMY_SP, Span, SyntaxContext};
use swc_core::ecma::ast::{
  ArrowExpr, BlockStmt, BlockStmtOrExpr, CallExpr, Callee, Expr, Function,
};

use crate::errors::set_transform_span;
use crate::types::Metadata;

/// Formats a diagnostic message that mirrors Babel's `buildCodeFrameError` helper.
///
/// The message mirrors the original behaviour by appending ` (line:column).` to the
/// provided error when a span is available. When no span is available the message is
/// returned unchanged with a trailing period.
pub fn build_code_frame_error(message: &str, span: Option<Span>, meta: &Metadata) -> String {
  let resolved_span = span.or(meta.own_span).or(meta.parent_span);
  if let Some(span) = resolved_span {
    // Capture span for panic diagnostics so TransformError::from_panic can attach it.
    set_transform_span(span);
  }

  format!("{message}.")
}

/// Wraps the provided block or expression in an IIFE.
pub fn wrap_node_in_iife(body: BlockStmtOrExpr) -> Expr {
  Expr::Call(CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Expr(Box::new(Expr::Arrow(ArrowExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      params: Vec::new(),
      body: Box::new(body),
      is_generator: false,
      is_async: false,
      type_params: None,
      return_type: None,
    }))),
    args: Vec::new(),
    type_args: None,
  })
}

fn block_to_block_stmt_or_expr(block: &BlockStmt) -> BlockStmtOrExpr {
  BlockStmtOrExpr::BlockStmt(block.clone())
}

fn clone_arrow_expr_body(body: &BlockStmtOrExpr) -> Expr {
  match body {
    BlockStmtOrExpr::BlockStmt(block) => {
      wrap_node_in_iife(BlockStmtOrExpr::BlockStmt(block.clone()))
    }
    BlockStmtOrExpr::Expr(expr) => *expr.clone(),
  }
}

fn clone_function_body(function: &Function) -> Expr {
  match &function.body {
    Some(body) => wrap_node_in_iife(block_to_block_stmt_or_expr(body)),
    None => Expr::Call(CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Arrow(ArrowExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        params: Vec::new(),
        body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          stmts: Vec::new(),
        })),
        is_generator: false,
        is_async: false,
        type_params: None,
        return_type: None,
      }))),
      args: Vec::new(),
      type_args: None,
    }),
  }
}

/// Mirrors Babel's `pickFunctionBody` helper by returning the expression body for
/// functions and wrapping block statements in an IIFE.
pub fn pick_function_body(expr: &Expr) -> Expr {
  match expr {
    Expr::Arrow(arrow) => clone_arrow_expr_body(&arrow.body),
    Expr::Fn(fn_expr) => clone_function_body(&fn_expr.function),
    _ => expr.clone(),
  }
}

#[cfg(test)]
mod tests {
  use super::{build_code_frame_error, pick_function_body, wrap_node_in_iife};
  use crate::types::{
    Metadata, PluginOptions, TransformFile, TransformFileOptions, TransformState,
  };
  use std::cell::RefCell;
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{BytePos, DUMMY_SP, FileName, SourceMap, Span, SyntaxContext};
  use swc_core::ecma::ast::{
    ArrowExpr, BlockStmt, BlockStmtOrExpr, CallExpr, Callee, Expr, FnExpr, Function, Ident,
    ReturnStmt, Stmt,
  };
  use swc_core::ecma::atoms::Atom;

  fn create_metadata_with_file() -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::transform_compiled_with_options(
      cm.clone(),
      Vec::new(),
      TransformFileOptions {
        filename: Some("test.tsx".into()),
        loc_filename: Some("test.tsx".into()),
        ..TransformFileOptions::default()
      },
    );
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));
    Metadata::new(state)
  }

  fn arrow_expr(body: BlockStmtOrExpr) -> Expr {
    Expr::Arrow(ArrowExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      params: Vec::new(),
      body: Box::new(body),
      is_generator: false,
      is_async: false,
      type_params: None,
      return_type: None,
    })
  }

  #[test]
  fn wrap_iife_wraps_expression() {
    let expr = wrap_node_in_iife(BlockStmtOrExpr::Expr(Box::new(Expr::Ident(Ident::new(
      Atom::from("a"),
      DUMMY_SP,
      SyntaxContext::empty(),
    )))));

    match expr {
      Expr::Call(CallExpr {
        callee: Callee::Expr(callee),
        ..
      }) => match *callee {
        Expr::Arrow(ArrowExpr { body, .. }) => match *body {
          BlockStmtOrExpr::Expr(inner) => match *inner {
            Expr::Ident(Ident { sym, .. }) => assert_eq!(sym.as_ref(), "a"),
            other => panic!("unexpected inner expression: {other:?}"),
          },
          other => panic!("unexpected IIFE body: {other:?}"),
        },
        other => panic!("unexpected callee: {other:?}"),
      },
      other => panic!("unexpected node: {other:?}"),
    }
  }

  #[test]
  fn pick_function_body_returns_expression_for_arrow() {
    let expr = arrow_expr(BlockStmtOrExpr::Expr(Box::new(Expr::Ident(Ident::new(
      Atom::from("value"),
      DUMMY_SP,
      SyntaxContext::empty(),
    )))));
    let result = pick_function_body(&expr);

    match result {
      Expr::Ident(Ident { sym, .. }) => assert_eq!(sym.as_ref(), "value"),
      other => panic!("unexpected expression: {other:?}"),
    }
  }

  #[test]
  fn pick_function_body_wraps_block_in_iife() {
    let block = BlockStmt {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      stmts: vec![Stmt::Return(ReturnStmt {
        span: DUMMY_SP,
        arg: Some(Box::new(Expr::Ident(Ident::new(
          Atom::from("value"),
          DUMMY_SP,
          SyntaxContext::empty(),
        )))),
      })],
    };
    let expr = arrow_expr(BlockStmtOrExpr::BlockStmt(block.clone()));
    let result = pick_function_body(&expr);

    match result {
      Expr::Call(CallExpr {
        callee: Callee::Expr(callee),
        ..
      }) => match *callee {
        Expr::Arrow(ArrowExpr { body, .. }) => match *body {
          BlockStmtOrExpr::BlockStmt(inner_block) => {
            assert_eq!(inner_block.stmts.len(), 1);
          }
          other => panic!("unexpected body: {other:?}"),
        },
        other => panic!("unexpected callee: {other:?}"),
      },
      other => panic!("unexpected node: {other:?}"),
    }
  }

  #[test]
  fn pick_function_body_handles_function_expressions() {
    let function = Expr::Fn(FnExpr {
      ident: None,
      function: Box::new(Function {
        params: Vec::new(),
        decorators: Vec::new(),
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        body: Some(BlockStmt {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          stmts: vec![Stmt::Return(ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(Expr::Ident(Ident::new(
              Atom::from("a"),
              DUMMY_SP,
              SyntaxContext::empty(),
            )))),
          })],
        }),
        is_async: false,
        is_generator: false,
        type_params: None,
        return_type: None,
      }),
    });

    let result = pick_function_body(&function);

    match result {
      Expr::Call(CallExpr {
        callee: Callee::Expr(callee),
        ..
      }) => match *callee {
        Expr::Arrow(ArrowExpr { body, .. }) => match *body {
          BlockStmtOrExpr::BlockStmt(block) => {
            assert_eq!(block.stmts.len(), 1);
          }
          other => panic!("unexpected body: {other:?}"),
        },
        other => panic!("unexpected callee: {other:?}"),
      },
      other => panic!("unexpected result: {other:?}"),
    }
  }

  #[test]
  fn build_code_frame_error_formats_location() {
    let meta = create_metadata_with_file();
    let state = meta.state();
    let sm = &state.file().source_map;
    let file = sm.new_source_file(
      Lrc::new(FileName::Custom("virtual.tsx".into())),
      "let x = 1;".to_string(),
    );
    drop(state);

    let span = Span::new(file.start_pos, file.start_pos + BytePos(3));
    let message = build_code_frame_error("Example error", Some(span), &meta);

    assert_eq!(message, "Example error.");
  }
}
