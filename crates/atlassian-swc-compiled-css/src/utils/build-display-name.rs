use swc_core::atoms::Atom;
use swc_core::common::{SyntaxContext, DUMMY_SP};
use swc_core::ecma::ast::{
  AssignExpr, AssignOp, AssignTarget, BinExpr, BinaryOp, BlockStmt, Expr, ExprStmt, Ident,
  IdentName, Lit, MemberExpr, MemberProp, Stmt, Str,
};

fn ident(name: &str) -> Ident {
  Ident::new(Atom::from(name), DUMMY_SP, SyntaxContext::empty())
}

fn ident_name(name: &str) -> IdentName {
  IdentName::new(Atom::from(name), DUMMY_SP)
}

fn string_literal(value: &str) -> Expr {
  Expr::Lit(Lit::Str(Str {
    span: DUMMY_SP,
    value: value.into(),
    raw: None,
  }))
}

fn member_expr(object: Expr, property: &str) -> MemberExpr {
  MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(object),
    prop: MemberProp::Ident(ident_name(property)),
  }
}

fn member_expr_expr(object: Expr, property: &str) -> Expr {
  Expr::Member(member_expr(object, property))
}

/// Mirrors the Babel helper by assigning a development-only `displayName`
/// property to the provided identifier.
pub fn build_display_name(identifier: &str, display_name: Option<&str>) -> Stmt {
  let display_name = display_name.unwrap_or(identifier);

  let process_env = member_expr_expr(Expr::Ident(ident("process")), "env");
  let node_env = member_expr_expr(process_env, "NODE_ENV");
  let production_literal = string_literal("production");

  let condition = Expr::Bin(BinExpr {
    span: DUMMY_SP,
    op: BinaryOp::NotEqEq,
    left: Box::new(node_env),
    right: Box::new(production_literal),
  });

  let display_member = member_expr(Expr::Ident(ident(identifier)), "displayName");

  let display_name_assignment = Expr::Assign(AssignExpr {
    span: DUMMY_SP,
    op: AssignOp::Assign,
    left: AssignTarget::from(display_member),
    right: Box::new(string_literal(display_name)),
  });

  Stmt::If(swc_core::ecma::ast::IfStmt {
    span: DUMMY_SP,
    test: Box::new(condition),
    cons: Box::new(Stmt::Block(BlockStmt {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      stmts: vec![Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(display_name_assignment),
      })],
    })),
    alt: None,
  })
}

#[cfg(test)]
mod tests {
  use super::build_display_name;
  use swc_core::ecma::ast::{BinaryOp, Expr, ExprStmt, Stmt};

  fn unwrap_if(stmt: Stmt) -> swc_core::ecma::ast::IfStmt {
    match stmt {
      Stmt::If(if_stmt) => if_stmt,
      other => panic!("expected if statement, found {:?}", other),
    }
  }

  #[test]
  fn sets_display_name_literal() {
    let stmt = build_display_name("Component", Some("MyComponent"));
    let if_stmt = unwrap_if(stmt);

    match *if_stmt.test {
      Expr::Bin(bin) => {
        assert_eq!(bin.op, BinaryOp::NotEqEq);
      }
      other => panic!("expected binary expression, found {:?}", other),
    }

    let block = if let Stmt::Block(block) = *if_stmt.cons {
      block
    } else {
      panic!("expected block statement");
    };

    assert_eq!(block.stmts.len(), 1);
    let Stmt::Expr(ExprStmt { expr, .. }) = &block.stmts[0] else {
      panic!("expected expression statement");
    };

    match &**expr {
      Expr::Assign(assign) => match &*assign.right {
        Expr::Lit(lit) => {
          if let swc_core::ecma::ast::Lit::Str(str_lit) = lit {
            assert_eq!(str_lit.value, "MyComponent");
          } else {
            panic!("expected string literal");
          }
        }
        other => panic!("expected string literal, found {:?}", other),
      },
      other => panic!("expected assignment expression, found {:?}", other),
    }
  }

  #[test]
  fn defaults_display_name_to_identifier() {
    let stmt = build_display_name("Button", None);
    let if_stmt = unwrap_if(stmt);

    let block = if let Stmt::Block(block) = *if_stmt.cons {
      block
    } else {
      panic!("expected block statement");
    };

    let Stmt::Expr(ExprStmt { expr, .. }) = &block.stmts[0] else {
      panic!("expected expression statement");
    };

    match &**expr {
      Expr::Assign(assign) => match &*assign.right {
        Expr::Lit(lit) => {
          if let swc_core::ecma::ast::Lit::Str(str_lit) = lit {
            assert_eq!(str_lit.value, "Button");
          } else {
            panic!("expected string literal");
          }
        }
        other => panic!("expected string literal, found {:?}", other),
      },
      other => panic!("expected assignment expression, found {:?}", other),
    }
  }
}
