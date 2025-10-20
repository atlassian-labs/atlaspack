use swc_core::{
  common::{DUMMY_SP, SyntaxContext},
  ecma::ast::*,
};

use crate::CompiledCssInJsCollector;

impl CompiledCssInJsCollector {
  pub fn build_display_if(name: &str) -> ModuleItem {
    let cond = Expr::Bin(BinExpr {
      span: DUMMY_SP,
      op: BinaryOp::NotEqEq,
      left: Box::new(Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Member(MemberExpr {
          span: DUMMY_SP,
          obj: Box::new(Expr::Ident(Ident::new(
            "process".into(),
            DUMMY_SP,
            SyntaxContext::empty(),
          ))),
          prop: MemberProp::Ident(
            Ident::new("env".into(), DUMMY_SP, SyntaxContext::empty()).into(),
          ),
        })),
        prop: MemberProp::Ident(
          Ident::new("NODE_ENV".into(), DUMMY_SP, SyntaxContext::empty()).into(),
        ),
      })),
      right: Box::new(Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: "production".into(),
        raw: None,
      }))),
    });
    let assign = Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: Box::new(Expr::Assign(AssignExpr {
        span: DUMMY_SP,
        op: AssignOp::Assign,
        left: AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
          span: DUMMY_SP,
          obj: Box::new(Expr::Ident(Ident::new(
            name.into(),
            DUMMY_SP,
            SyntaxContext::empty(),
          ))),
          prop: MemberProp::Ident(
            Ident::new("displayName".into(), DUMMY_SP, SyntaxContext::empty()).into(),
          ),
        })),
        right: Box::new(Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          value: name.into(),
          raw: None,
        }))),
      })),
    });
    ModuleItem::Stmt(Stmt::If(IfStmt {
      span: DUMMY_SP,
      test: Box::new(cond),
      cons: Box::new(Stmt::Block(BlockStmt {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        stmts: vec![assign],
      })),
      alt: None,
    }))
  }
}
