use swc_core::common::{Span, Spanned};
use swc_core::ecma::ast::{
  ClassDecl, ClassExpr, Decl, DefaultDecl, ExportNamedSpecifier, ExportSpecifier, Expr, FnDecl,
  FnExpr, ImportDecl, ImportSpecifier, Module, ModuleDecl, ModuleExportName, ModuleItem,
  ObjectPatProp, Pat, Stmt, VarDecl, VarDeclKind,
};

use crate::types::{Metadata, SharedTransformState};
use crate::utils_types::{BindingPath, BindingSource, ImportBindingKind, PartialBindingWithMeta};

fn prop_name_to_string(name: &swc_core::ecma::ast::PropName) -> Option<String> {
  use swc_core::ecma::ast::PropName;

  match name {
    PropName::Ident(ident) => Some(ident.sym.to_string()),
    PropName::Str(value) => Some(value.value.to_string()),
    PropName::Num(value) => Some(value.value.to_string()),
    PropName::BigInt(value) => Some(value.value.to_string()),
    PropName::Computed(_) => None,
  }
}

fn insert_module_binding(
  metadata: &Metadata,
  name: &str,
  node: Option<Expr>,
  path: Option<BindingPath>,
  constant: bool,
  source: BindingSource,
) {
  let binding = PartialBindingWithMeta::new(node, path, constant, metadata.clone(), source);
  if std::env::var("STACK_DEBUG_SCOPE").is_ok() {
    let kind = binding
      .path
      .as_ref()
      .map(|p| format!("{:?}", p.kind))
      .unwrap_or_else(|| "None".into());
    eprintln!(
      "[module_scope] insert name='{}' kind={} constant={}",
      name, kind, constant
    );
  }
  metadata.insert_parent_binding(name, binding);
}

fn module_export_name_to_string(name: &ModuleExportName) -> String {
  match name {
    ModuleExportName::Ident(ident) => ident.sym.to_string(),
    ModuleExportName::Str(value) => value.value.to_string(),
  }
}

fn register_export_named_specifier(
  metadata: &Metadata,
  source: &str,
  specifier: &ExportNamedSpecifier,
) {
  if specifier.is_type_only {
    return;
  }

  let exported = specifier
    .exported
    .as_ref()
    .map(module_export_name_to_string)
    .unwrap_or_else(|| module_export_name_to_string(&specifier.orig));

  let import_name = module_export_name_to_string(&specifier.orig);
  let import_kind = if import_name == "default" {
    ImportBindingKind::Default
  } else {
    ImportBindingKind::Named(import_name)
  };

  let path = BindingPath::import(Some(specifier.span), source.to_string(), import_kind);
  insert_module_binding(
    metadata,
    &exported,
    None,
    Some(path),
    true,
    BindingSource::Import,
  );
}

fn register_export_namespace_specifier(
  metadata: &Metadata,
  source: &str,
  specifier: &swc_core::ecma::ast::ExportNamespaceSpecifier,
) {
  let exported = module_export_name_to_string(&specifier.name);
  let path = BindingPath::import(
    Some(specifier.span),
    source.to_string(),
    ImportBindingKind::Namespace,
  );

  insert_module_binding(
    metadata,
    &exported,
    None,
    Some(path),
    true,
    BindingSource::Import,
  );
}

fn register_export_default_specifier(
  metadata: &Metadata,
  source: &str,
  specifier: &swc_core::ecma::ast::ExportDefaultSpecifier,
) {
  let exported = specifier.exported.sym.to_string();
  let path = BindingPath::import(
    Some(specifier.exported.span),
    source.to_string(),
    ImportBindingKind::Default,
  );

  insert_module_binding(
    metadata,
    &exported,
    None,
    Some(path),
    true,
    BindingSource::Import,
  );
}

fn register_export_named_decl(metadata: &Metadata, decl: &swc_core::ecma::ast::NamedExport) {
  if let Some(source) = decl.src.as_ref() {
    let source_value = source.value.to_string();

    for specifier in &decl.specifiers {
      match specifier {
        ExportSpecifier::Named(named) => {
          register_export_named_specifier(metadata, &source_value, named)
        }
        ExportSpecifier::Namespace(namespace) => {
          register_export_namespace_specifier(metadata, &source_value, namespace)
        }
        ExportSpecifier::Default(default_specifier) => {
          register_export_default_specifier(metadata, &source_value, default_specifier)
        }
      }
    }

    return;
  }

  // No source provided: local export aliases are handled during binding
  // resolution to mirror Babel behaviour (see resolve_binding).
}

fn register_function_decl(metadata: &Metadata, decl: &FnDecl) {
  let name = decl.ident.sym.as_ref();
  let expr = Expr::Fn(FnExpr {
    ident: Some(decl.ident.clone()),
    function: decl.function.clone(),
  });
  let path = BindingPath::new(Some(decl.ident.span));

  insert_module_binding(
    metadata,
    name,
    Some(expr),
    Some(path),
    true,
    BindingSource::Module,
  );
}

fn register_class_decl(metadata: &Metadata, decl: &ClassDecl) {
  let name = decl.ident.sym.as_ref();
  let expr = Expr::Class(ClassExpr {
    ident: Some(decl.ident.clone()),
    class: decl.class.clone(),
  });
  let path = BindingPath::new(Some(decl.ident.span));

  insert_module_binding(
    metadata,
    name,
    Some(expr),
    Some(path),
    true,
    BindingSource::Module,
  );
}

fn register_pattern(
  pattern: &Pat,
  init: Option<&Expr>,
  constant: bool,
  path: Vec<String>,
  default_value: Option<Expr>,
  metadata: &Metadata,
  span: Option<Span>,
) {
  match pattern {
    Pat::Ident(binding) => {
      let name = binding.id.sym.as_ref();
      let span = span.unwrap_or(binding.id.span);
      // Treat all variable declarators as `BindingPathKind::Variable` to mirror Babel's
      // path metadata, even when there is no nested destructuring. This allows downstream
      // resolution to apply variable-specific compat hooks (e.g., avoiding string inlining
      // for imported shorthands).
      let binding_path = Some(BindingPath::variable(Some(span), path, default_value));

      insert_module_binding(
        metadata,
        name,
        init.cloned(),
        binding_path,
        constant,
        BindingSource::Module,
      );
    }
    Pat::Assign(assign) => {
      let assign_default = Some(*assign.right.clone()).or(default_value);

      register_pattern(
        &assign.left,
        init,
        constant,
        path,
        assign_default,
        metadata,
        Some(assign.span),
      );
    }
    Pat::Object(object) => {
      for prop in &object.props {
        match prop {
          ObjectPatProp::KeyValue(key_value) => {
            if let Some(key) = prop_name_to_string(&key_value.key) {
              let mut next_path = path.clone();
              next_path.push(key);

              register_pattern(
                &key_value.value,
                init,
                constant,
                next_path,
                None,
                metadata,
                Some(key_value.value.span()),
              );
            }
          }
          ObjectPatProp::Assign(assign) => {
            let mut next_path = path.clone();
            next_path.push(assign.key.sym.to_string());
            let default_expr = assign.value.as_ref().map(|expr| expr.as_ref().clone());
            let span = Some(assign.key.span);
            let binding_path = BindingPath::variable(span, next_path, default_expr);

            insert_module_binding(
              metadata,
              assign.key.sym.as_ref(),
              init.cloned(),
              Some(binding_path),
              constant,
              BindingSource::Module,
            );
          }
          ObjectPatProp::Rest(rest) => {
            register_pattern(
              &rest.arg,
              init,
              constant,
              Vec::new(),
              None,
              metadata,
              Some(rest.span),
            );
          }
        }
      }
    }
    _ => {}
  }
}

fn register_var_decl(metadata: &Metadata, decl: &VarDecl) {
  let constant = matches!(decl.kind, VarDeclKind::Const);

  for declarator in &decl.decls {
    let init = declarator.init.as_deref();
    register_pattern(
      &declarator.name,
      init,
      constant,
      Vec::new(),
      None,
      metadata,
      Some(declarator.span),
    );
  }
}

fn register_decl_for_scope(metadata: &Metadata, decl: &Decl) {
  match decl {
    Decl::Var(var_decl) => register_var_decl(metadata, var_decl),
    Decl::Fn(fn_decl) => register_function_decl(metadata, fn_decl),
    Decl::Class(class_decl) => register_class_decl(metadata, class_decl),
    _ => {}
  }
}

fn register_stmt_for_scope(metadata: &Metadata, stmt: &Stmt) {
  if let Stmt::Decl(decl) = stmt {
    register_decl_for_scope(metadata, decl);
  }
}

fn register_import_decl_for_scope(metadata: &Metadata, import: &ImportDecl) {
  let source = import.src.value.to_string();

  for specifier in &import.specifiers {
    match specifier {
      ImportSpecifier::Named(named) => {
        let local = named.local.sym.as_ref();
        let kind = match &named.imported {
          Some(ModuleExportName::Ident(ident)) if ident.sym.as_ref() == "default" => {
            ImportBindingKind::Default
          }
          Some(ModuleExportName::Ident(ident)) => ImportBindingKind::Named(ident.sym.to_string()),
          Some(ModuleExportName::Str(value)) => ImportBindingKind::Named(value.value.to_string()),
          None => ImportBindingKind::Named(local.to_string()),
        };

        let path = BindingPath::import(Some(named.span), source.clone(), kind);
        insert_module_binding(
          metadata,
          local,
          None,
          Some(path),
          true,
          BindingSource::Import,
        );
      }
      ImportSpecifier::Default(default_specifier) => {
        let local = default_specifier.local.sym.as_ref();
        let path = BindingPath::import(
          Some(default_specifier.span),
          source.clone(),
          ImportBindingKind::Default,
        );
        insert_module_binding(
          metadata,
          local,
          None,
          Some(path),
          true,
          BindingSource::Import,
        );
      }
      ImportSpecifier::Namespace(namespace) => {
        let local = namespace.local.sym.as_ref();
        let path = BindingPath::import(
          Some(namespace.span),
          source.clone(),
          ImportBindingKind::Namespace,
        );
        insert_module_binding(
          metadata,
          local,
          None,
          Some(path),
          true,
          BindingSource::Import,
        );
      }
    }
  }
}

fn register_module_decl_for_scope(metadata: &Metadata, decl: &ModuleDecl) {
  match decl {
    ModuleDecl::Import(import) => register_import_decl_for_scope(metadata, import),
    ModuleDecl::ExportNamed(named) => register_export_named_decl(metadata, named),
    ModuleDecl::ExportDecl(export_decl) => register_decl_for_scope(metadata, &export_decl.decl),
    ModuleDecl::ExportDefaultDecl(default_decl) => match &default_decl.decl {
      DefaultDecl::Fn(fn_expr) => {
        if let Some(ident) = &fn_expr.ident {
          let expr = Expr::Fn(fn_expr.clone());
          let path = BindingPath::new(Some(default_decl.span));
          insert_module_binding(
            metadata,
            ident.sym.as_ref(),
            Some(expr),
            Some(path),
            true,
            BindingSource::Module,
          );
        }
      }
      DefaultDecl::Class(class_expr) => {
        if let Some(ident) = &class_expr.ident {
          let expr = Expr::Class(class_expr.clone());
          let path = BindingPath::new(Some(default_decl.span));
          insert_module_binding(
            metadata,
            ident.sym.as_ref(),
            Some(expr),
            Some(path),
            true,
            BindingSource::Module,
          );
        }
      }
      DefaultDecl::TsInterfaceDecl(_) => {}
    },
    _ => {}
  }
}

/// Populate the shared module scope on the provided transform state so that
/// downstream binding resolution can mirror the Babel behaviour.
pub fn populate_module_scope(state: &SharedTransformState, module: &Module) {
  {
    let state_mut = state.borrow_mut();
    state_mut.module_scope.borrow_mut().clear();
  }

  let metadata = Metadata::new(state.clone());

  for item in &module.body {
    match item {
      ModuleItem::ModuleDecl(decl) => register_module_decl_for_scope(&metadata, decl),
      ModuleItem::Stmt(stmt) => register_stmt_for_scope(&metadata, stmt),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::populate_module_scope;
  use crate::types::{PluginOptions, TransformFile, TransformState};
  use crate::utils_types::{BindingPathKind, BindingSource, ImportBindingKind};
  use std::cell::RefCell;
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{FileName, SourceMap};
  use swc_core::ecma::ast::Module;
  use swc_core::ecma::parser::lexer::Lexer;
  use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax};

  fn parse_module(code: &str) -> Module {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("test.ts".into()).into(), code.into());
    let lexer = Lexer::new(
      Syntax::Es(EsSyntax {
        jsx: true,
        ..Default::default()
      }),
      Default::default(),
      StringInput::from(&*fm),
      None,
    );

    let mut parser = Parser::new_from(lexer);
    match parser.parse_module() {
      Ok(module) => module,
      Err(err) => panic!("failed to parse module: {:?}", err),
    }
  }

  fn create_state() -> Rc<RefCell<TransformState>> {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::new(cm, Vec::new());
    Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )))
  }

  #[test]
  fn registers_reexported_specifiers() {
    let module = parse_module(
      "export { blue, default as primary } from './colors';\n\
             export * as theme from './theme';",
    );

    let state = create_state();
    populate_module_scope(&state, &module);

    let blue_binding = {
      let state_ref = state.borrow();
      let scope_ref = state_ref.module_scope.borrow();
      scope_ref.get("blue").cloned().expect("blue binding")
    };
    assert_eq!(blue_binding.source, BindingSource::Import);
    match blue_binding.path.as_ref().expect("binding path").kind {
      BindingPathKind::Import {
        ref source,
        ref kind,
      } => {
        assert_eq!(source, "./colors");
        assert!(matches!(kind, ImportBindingKind::Named(name) if name == "blue"));
      }
      _ => panic!("expected import binding"),
    }

    let primary_binding = {
      let state_ref = state.borrow();
      let scope_ref = state_ref.module_scope.borrow();
      scope_ref.get("primary").cloned().expect("primary binding")
    };
    match primary_binding.path.as_ref().expect("binding path").kind {
      BindingPathKind::Import {
        ref source,
        ref kind,
      } => {
        assert_eq!(source, "./colors");
        assert!(matches!(kind, ImportBindingKind::Default));
      }
      _ => panic!("expected import binding"),
    }

    let theme_binding = {
      let state_ref = state.borrow();
      let scope_ref = state_ref.module_scope.borrow();
      scope_ref.get("theme").cloned().expect("theme binding")
    };
    match theme_binding.path.as_ref().expect("binding path").kind {
      BindingPathKind::Import {
        ref source,
        ref kind,
      } => {
        assert_eq!(source, "./theme");
        assert!(matches!(kind, ImportBindingKind::Namespace));
      }
      _ => panic!("expected import binding"),
    }
  }
}
