use swc_core::ecma::ast::{
  ImportNamedSpecifier, ImportSpecifier, ModuleDecl, ModuleExportName, ModuleItem, Program,
};

use crate::types::TransformState;

fn imported_name(specifier: &ImportNamedSpecifier) -> &str {
  match &specifier.imported {
    Some(ModuleExportName::Ident(ident)) => ident.sym.as_ref(),
    Some(ModuleExportName::Str(value)) => value.value.as_ref(),
    None => specifier.local.sym.as_ref(),
  }
}

/// Traverse the module for `css` named imports and cache the discovered local
/// binding on the transform state. This mirrors the Babel helper so that
/// modules pulled in during binding resolution retain knowledge of runtime
/// aliases.
pub fn set_imported_compiled_imports(program: &Program, state: &mut TransformState) {
  let module = match program {
    Program::Module(module) => module,
    Program::Script(_) => return,
  };

  for item in &module.body {
    let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item else {
      continue;
    };

    for specifier in &import.specifiers {
      let ImportSpecifier::Named(named) = specifier else {
        continue;
      };

      if imported_name(named) == "css" {
        state.imported_compiled_imports.css = Some(named.local.sym.to_string());
        return;
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::set_imported_compiled_imports;
  use crate::types::{PluginOptions, TransformFile, TransformState};
  use swc_core::common::sync::Lrc;
  use swc_core::common::{DUMMY_SP, SourceMap, SyntaxContext};
  use swc_core::ecma::ast::{
    Ident, ImportDecl, ImportNamedSpecifier, ImportPhase, ImportSpecifier, Module, ModuleDecl,
    ModuleExportName, ModuleItem, Program, Str,
  };

  fn state() -> TransformState {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::new(cm, Vec::new());
    TransformState::new(file, PluginOptions::default())
  }

  fn module_with_import(specifier: ImportSpecifier) -> Program {
    Program::Module(Module {
      span: DUMMY_SP,
      body: vec![ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        span: DUMMY_SP,
        specifiers: vec![specifier],
        src: Box::new(Str {
          span: DUMMY_SP,
          value: "module".into(),
          raw: None,
        }),
        type_only: false,
        with: None,
        phase: ImportPhase::Evaluation,
      }))],
      shebang: None,
    })
  }

  #[test]
  fn records_direct_css_import() {
    let program = module_with_import(ImportSpecifier::Named(ImportNamedSpecifier {
      span: DUMMY_SP,
      local: Ident::new("css".into(), DUMMY_SP, SyntaxContext::empty()),
      imported: None,
      is_type_only: false,
    }));

    let mut state = state();
    set_imported_compiled_imports(&program, &mut state);

    assert_eq!(state.imported_compiled_imports.css.as_deref(), Some("css"));
  }

  #[test]
  fn records_css_alias() {
    let program = module_with_import(ImportSpecifier::Named(ImportNamedSpecifier {
      span: DUMMY_SP,
      local: Ident::new("alias".into(), DUMMY_SP, SyntaxContext::empty()),
      imported: Some(ModuleExportName::Ident(Ident::new(
        "css".into(),
        DUMMY_SP,
        SyntaxContext::empty(),
      ))),
      is_type_only: false,
    }));

    let mut state = state();
    set_imported_compiled_imports(&program, &mut state);

    assert_eq!(
      state.imported_compiled_imports.css.as_deref(),
      Some("alias")
    );
  }

  #[test]
  fn ignores_non_matching_imports() {
    let program = module_with_import(ImportSpecifier::Named(ImportNamedSpecifier {
      span: DUMMY_SP,
      local: Ident::new("other".into(), DUMMY_SP, SyntaxContext::empty()),
      imported: Some(ModuleExportName::Ident(Ident::new(
        "styled".into(),
        DUMMY_SP,
        SyntaxContext::empty(),
      ))),
      is_type_only: false,
    }));

    let mut state = state();
    set_imported_compiled_imports(&program, &mut state);

    assert!(state.imported_compiled_imports.css.is_none());
  }
}
