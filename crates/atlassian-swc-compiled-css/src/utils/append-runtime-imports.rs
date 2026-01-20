use std::collections::HashSet;

use swc_core::common::{DUMMY_SP, SyntaxContext};
use swc_core::ecma::ast::{
  Ident, ImportDecl, ImportNamedSpecifier, ImportPhase, ImportSpecifier, ModuleDecl, ModuleItem,
  Str,
};

use crate::types::TransformState;

const COMPILED_RUNTIME_MODULE: &str = "@compiled/react/runtime";
const COMPILED_RUNTIME_IMPORTS_WITH_COMPRESSION: &[&str] = &["ac", "ix", "CC", "CS"];
const COMPILED_RUNTIME_IMPORTS_WITHOUT_COMPRESSION: &[&str] = &["ax", "ix", "CC", "CS"];

fn runtime_imports<'a>(state: &'a TransformState) -> &'a [&'static str] {
  let uses_compression = state
    .opts
    .class_name_compression_map
    .as_ref()
    .map(|map| !map.is_empty())
    .unwrap_or(false);

  if uses_compression {
    COMPILED_RUNTIME_IMPORTS_WITH_COMPRESSION
  } else {
    COMPILED_RUNTIME_IMPORTS_WITHOUT_COMPRESSION
  }
}

fn local_name(specifier: &ImportSpecifier) -> &str {
  match specifier {
    ImportSpecifier::Named(named) => named.local.sym.as_ref(),
    ImportSpecifier::Default(default) => default.local.sym.as_ref(),
    ImportSpecifier::Namespace(namespace) => namespace.local.sym.as_ref(),
  }
}

fn create_specifier(name: &str) -> ImportSpecifier {
  ImportSpecifier::Named(ImportNamedSpecifier {
    span: DUMMY_SP,
    local: Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty()),
    imported: None,
    is_type_only: false,
  })
}

fn ensure_runtime_import(module: &mut swc_core::ecma::ast::Module, state: &mut TransformState) {
  let runtime_imports = runtime_imports(state);

  if runtime_imports.is_empty() {
    return;
  }

  if let Some(existing) = module.body.iter_mut().find_map(|item| match item {
    ModuleItem::ModuleDecl(ModuleDecl::Import(import))
      if import.src.value.as_ref() == COMPILED_RUNTIME_MODULE =>
    {
      Some(import)
    }
    _ => None,
  }) {
    let mut local_names: HashSet<String> = existing
      .specifiers
      .iter()
      .map(|specifier| local_name(specifier).to_string())
      .collect();

    for import in runtime_imports.iter().copied() {
      if local_names.insert(import.to_string()) {
        existing.specifiers.push(create_specifier(import));
      }
    }

    return;
  }

  let specifiers = runtime_imports
    .iter()
    .map(|name| create_specifier(name))
    .collect::<Vec<_>>();

  let import_decl = ImportDecl {
    span: DUMMY_SP,
    specifiers,
    src: Box::new(Str {
      span: DUMMY_SP,
      value: COMPILED_RUNTIME_MODULE.into(),
      raw: None,
    }),
    type_only: false,
    with: None,
    phase: ImportPhase::Evaluation,
  };

  module
    .body
    .insert(0, ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)));
}

/// Append the Compiled runtime import to the program, matching the Babel helper
/// by augmenting an existing declaration or creating a new one when necessary.
pub fn append_runtime_imports(
  module: &mut swc_core::ecma::ast::Module,
  state: &mut TransformState,
) {
  ensure_runtime_import(module, state);
}

#[cfg(test)]
mod tests {
  use super::append_runtime_imports;
  use crate::types::{PluginOptions, TransformFile, TransformState};
  use swc_core::common::sync::Lrc;
  use swc_core::common::{FileName, SourceMap};
  use swc_core::ecma::ast::{ImportSpecifier, Module, ModuleDecl, ModuleItem};
  use swc_core::ecma::parser::lexer::Lexer;
  use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax};

  fn create_state(options: PluginOptions) -> TransformState {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::new(cm, Vec::new());
    TransformState::new(file, options)
  }

  fn parse_module(code: &str) -> Module {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("test.js".into()).into(), code.to_string());
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
    parser.parse_module().expect("parse module")
  }

  fn local_names(module: &Module) -> Vec<String> {
    let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = &module.body[0] else {
      panic!("expected runtime import");
    };

    import
      .specifiers
      .iter()
      .map(|specifier| match specifier {
        ImportSpecifier::Named(named) => named.local.sym.to_string(),
        ImportSpecifier::Default(default) => default.local.sym.to_string(),
        ImportSpecifier::Namespace(namespace) => namespace.local.sym.to_string(),
      })
      .collect()
  }

  #[test]
  fn inserts_runtime_import_when_missing() {
    let mut module = parse_module("const value = 1;");
    let mut state = create_state(PluginOptions::default());

    append_runtime_imports(&mut module, &mut state);

    match &module.body[0] {
      ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
        assert_eq!(import.src.value.as_ref(), "@compiled/react/runtime");
        assert_eq!(local_names(&module), vec!["ax", "ix", "CC", "CS"]);
      }
      _ => panic!("expected runtime import declaration"),
    }
  }

  #[test]
  fn appends_missing_specifiers_to_existing_import() {
    let mut module =
      parse_module("import { ix } from '@compiled/react/runtime';\nconst value = 1;");
    let mut state = create_state(PluginOptions::default());

    append_runtime_imports(&mut module, &mut state);

    assert_eq!(local_names(&module), vec!["ix", "ax", "CC", "CS"]);
  }

  #[test]
  fn respects_compression_map() {
    let mut module = parse_module("const value = 1;");
    let mut options = PluginOptions::default();
    let mut map = std::collections::BTreeMap::new();
    map.insert("a".to_string(), "b".to_string());
    options.class_name_compression_map = Some(map);
    let mut state = create_state(options);

    append_runtime_imports(&mut module, &mut state);

    assert_eq!(local_names(&module), vec!["ac", "ix", "CC", "CS"]);
  }

  #[test]
  fn preserves_existing_aliases_without_duplicates() {
    let mut module =
      parse_module("import { CC as CompiledRoot, ix } from '@compiled/react/runtime';");
    let mut state = create_state(PluginOptions::default());

    append_runtime_imports(&mut module, &mut state);

    assert_eq!(
      local_names(&module),
      vec!["CompiledRoot", "ix", "ax", "CC", "CS"]
    );
  }
}
