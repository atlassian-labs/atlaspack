use swc_core::common::Spanned;
use swc_core::ecma::ast::{
  Decl, DefaultDecl, ExportDecl, ExportSpecifier, Expr, Ident, Module, ModuleDecl,
  ModuleExportName, ModuleItem, Program, VarDecl, VarDeclarator,
};

use crate::utils_traversers_types::TraverserResult;

fn program_module<'a>(program: &'a Program) -> Option<&'a Module> {
  match program {
    Program::Module(module) => Some(module),
    Program::Script(_) => None,
  }
}

fn default_decl_to_expr(decl: &DefaultDecl) -> Option<Expr> {
  match decl {
    DefaultDecl::Fn(fn_expr) => Some(Expr::Fn(fn_expr.clone())),
    DefaultDecl::Class(class_expr) => Some(Expr::Class(class_expr.clone())),
    DefaultDecl::TsInterfaceDecl(_) => None,
  }
}

fn module_export_name_to_ident(name: &ModuleExportName) -> Option<Ident> {
  match name {
    ModuleExportName::Ident(ident) => Some(ident.clone()),
    ModuleExportName::Str(_) => None,
  }
}

fn named_specifier_matches(
  specifier: &ExportSpecifier,
  export_name: &str,
) -> Option<TraverserResult<Expr>> {
  match specifier {
    ExportSpecifier::Named(named) => {
      let exported = named.exported.as_ref().unwrap_or(&named.orig);

      if let ModuleExportName::Ident(ident) = exported {
        if ident.sym.as_ref() == export_name {
          return Some(TraverserResult {
            node: Expr::Ident(ident.clone()),
            span: named.span,
          });
        }
      }

      None
    }
    ExportSpecifier::Default(export_default) => {
      if export_name == "default" {
        Some(TraverserResult {
          node: Expr::Ident(export_default.exported.clone()),
          span: export_default.exported.span,
        })
      } else {
        None
      }
    }
    ExportSpecifier::Namespace(namespace) => {
      if let ModuleExportName::Ident(ident) = &namespace.name {
        if ident.sym.as_ref() == export_name {
          return Some(TraverserResult {
            node: Expr::Ident(ident.clone()),
            span: namespace.span,
          });
        }
      }

      None
    }
  }
}

fn var_decl_matches(var: &VarDecl, export_name: &str) -> Option<TraverserResult<Expr>> {
  for declarator in &var.decls {
    if let Some(result) = var_declarator_matches(declarator, export_name) {
      return Some(result);
    }
  }

  None
}

fn var_declarator_matches(
  declarator: &VarDeclarator,
  export_name: &str,
) -> Option<TraverserResult<Expr>> {
  let swc_core::ecma::ast::Pat::Ident(binding) = &declarator.name else {
    return None;
  };

  let name = binding.id.sym.as_ref();

  if name != export_name {
    return None;
  }

  if let Some(init) = &declarator.init {
    Some(TraverserResult {
      node: (*init.clone()),
      span: init.span(),
    })
  } else {
    Some(TraverserResult {
      node: Expr::Ident(binding.id.clone()),
      span: declarator.span,
    })
  }
}

/// Find the default export of a module and return the exported expression with its span.
pub fn get_default_export(program: &Program) -> Option<TraverserResult<Expr>> {
  let module = program_module(program)?;

  for item in &module.body {
    let ModuleItem::ModuleDecl(decl) = item else {
      continue;
    };

    match decl {
      ModuleDecl::ExportDefaultDecl(default_decl) => {
        if let Some(expr) = default_decl_to_expr(&default_decl.decl) {
          return Some(TraverserResult {
            node: expr,
            span: default_decl.span,
          });
        }
      }
      ModuleDecl::ExportDefaultExpr(default_expr) => {
        return Some(TraverserResult {
          node: (*default_expr.expr).clone(),
          span: default_expr.span,
        });
      }
      ModuleDecl::ExportNamed(named) => {
        for specifier in &named.specifiers {
          let ExportSpecifier::Named(named_specifier) = specifier else {
            continue;
          };

          if let Some(exported) = named_specifier.exported.as_ref() {
            if let ModuleExportName::Ident(ident) = exported {
              if ident.sym.as_ref() == "default" {
                if let Some(local) = module_export_name_to_ident(&named_specifier.orig) {
                  return Some(TraverserResult {
                    node: Expr::Ident(local),
                    span: named_specifier.span,
                  });
                }
              }
            }
          }
        }
      }
      _ => {}
    }
  }

  None
}

fn export_decl_matches(
  export_decl: &ExportDecl,
  export_name: &str,
) -> Option<TraverserResult<Expr>> {
  match &export_decl.decl {
    Decl::Var(var) => var_decl_matches(var, export_name),
    _ => None,
  }
}

/// Find a named export in a module and return the exported node with its span.
pub fn get_named_export(program: &Program, export_name: &str) -> Option<TraverserResult<Expr>> {
  let module = program_module(program)?;

  for item in &module.body {
    let ModuleItem::ModuleDecl(decl) = item else {
      continue;
    };

    match decl {
      ModuleDecl::ExportDecl(export_decl) => {
        if let Some(result) = export_decl_matches(export_decl, export_name) {
          return Some(result);
        }
      }
      ModuleDecl::ExportNamed(named) => {
        for specifier in &named.specifiers {
          if let Some(result) = named_specifier_matches(specifier, export_name) {
            return Some(result);
          }
        }
      }
      _ => {}
    }
  }

  None
}

#[cfg(test)]
mod tests {
  use super::{get_default_export, get_named_export};
  use crate::utils_traversers_types::TraverserResult;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{FileName, SourceMap};
  use swc_core::ecma::ast::{EsVersion, Expr, Lit, Program, Str};
  use swc_core::ecma::parser::lexer::Lexer;
  use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax};

  fn parse_module(code: &str) -> Program {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("test.js".into()).into(), code.into());
    let lexer = Lexer::new(
      Syntax::Es(EsSyntax {
        jsx: true,
        ..Default::default()
      }),
      EsVersion::Es2022,
      StringInput::from(&*fm),
      None,
    );

    let mut parser = Parser::new_from(lexer);
    let module = parser.parse_module().expect("module should parse");
    Program::Module(module)
  }

  fn assert_string_literal(result: TraverserResult<Expr>, expected: &str) {
    match result.node {
      Expr::Lit(Lit::Str(Str { value, .. })) => assert_eq!(value.as_ref(), expected),
      _ => panic!("expected string literal"),
    }
  }

  #[test]
  fn finds_default_export_expression() {
    let program = parse_module("export default 'blue';");
    let result = get_default_export(&program).expect("default export");

    assert_string_literal(result, "blue");
  }

  #[test]
  fn finds_default_export_alias() {
    let program = parse_module("const color = 'red'; export { color as default };");
    let result = get_default_export(&program).expect("default export");

    match result.node {
      Expr::Ident(ident) => assert_eq!(ident.sym.as_ref(), "color"),
      _ => panic!("expected identifier"),
    }
  }

  #[test]
  fn finds_named_export_variable_initializer() {
    let program = parse_module("export const blue = 'blue';");
    let result = get_named_export(&program, "blue").expect("named export");

    assert_string_literal(result, "blue");
  }

  #[test]
  fn finds_named_export_specifier_alias() {
    let program = parse_module("const color = 'red'; export { color as primary };");
    let result = get_named_export(&program, "primary").expect("named export");

    match result.node {
      Expr::Ident(ident) => assert_eq!(ident.sym.as_ref(), "primary"),
      _ => panic!("expected identifier"),
    }
  }
}
