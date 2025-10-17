use swc_core::{
  common::{DUMMY_SP, SyntaxContext},
  ecma::ast::*,
};

use crate::AtomicCssCollector;

impl AtomicCssCollector {
  fn alloc_unique_ident_for(&mut self, preferred: &str, fallback: &str) -> Ident {
    let cand = if !self.used_idents.contains(preferred) {
      preferred
    } else {
      fallback
    };
    if !self.used_idents.contains(cand) {
      self.used_idents.insert(cand.to_string());
      return Ident::new(cand.into(), DUMMY_SP, SyntaxContext::empty());
    }
    // add numeric suffixes
    let mut idx = 2usize;
    loop {
      let name = format!("{}{}", fallback, idx);
      if !self.used_idents.contains(&name) {
        self.used_idents.insert(name.clone());
        return Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty());
      }
      idx += 1;
    }
  }
}

// Helper functions for common import operations
fn create_ident(name: &str) -> Ident {
  Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty())
}

fn create_import_specifier(local_name: &str, imported_name: Option<&str>) -> ImportSpecifier {
  let local = create_ident(local_name);
  let imported = imported_name.map(|name| ModuleExportName::Ident(create_ident(name)));

  ImportSpecifier::Named(ImportNamedSpecifier {
    span: DUMMY_SP,
    local,
    imported,
    is_type_only: false,
  })
}

fn create_import_decl(module_path: &str, specifiers: Vec<ImportSpecifier>) -> ModuleItem {
  ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers,
    src: Box::new(Str {
      span: DUMMY_SP,
      value: module_path.into(),
      raw: None,
    }),
    type_only: false,
    with: None,
    phase: Default::default(),
  }))
}

fn find_existing_import_specifier(m: &Module, module_path: &str, specifier_name: &str) -> bool {
  m.body.iter().any(|item| {
    if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = item
      && &*import_decl.src.value == module_path
    {
      return import_decl.specifiers.iter().any(|spec| {
        if let ImportSpecifier::Named(named) = spec {
          match &named.imported {
            Some(ModuleExportName::Ident(id)) => &*id.sym == specifier_name,
            Some(ModuleExportName::Str(s)) => &*s.value == specifier_name,
            None => &*named.local.sym == specifier_name,
          }
        } else {
          false
        }
      });
    }
    false
  })
}

fn add_to_existing_import(m: &mut Module, module_path: &str, specifier: ImportSpecifier) -> bool {
  for item in &mut m.body {
    if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = item
      && &*import_decl.src.value == module_path
    {
      import_decl.specifiers.push(specifier);
      return true;
    }
  }
  false
}

fn prepend_import(m: &mut Module, import: ModuleItem) {
  let mut items = Vec::with_capacity(m.body.len() + 1);
  items.push(import);
  items.append(&mut m.body);
  m.body = items;
}

pub fn ensure_forward_ref_import(visitor: &mut AtomicCssCollector, m: &mut Module) {
  if !visitor.need_forward_ref {
    return;
  }

  // Check if forwardRef is already imported and get the local identifier
  for item in &m.body {
    if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = item
      && &*import_decl.src.value == "react"
    {
      for spec in &import_decl.specifiers {
        if let ImportSpecifier::Named(named) = spec {
          let is_forward_ref = match &named.imported {
            Some(ModuleExportName::Ident(id)) => &*id.sym == "forwardRef",
            Some(ModuleExportName::Str(s)) => &*s.value == "forwardRef",
            None => &*named.local.sym == "forwardRef",
          };
          if is_forward_ref {
            visitor.forward_ref_ident = Some(named.local.clone());
            visitor.used_idents.insert(named.local.sym.to_string());
            visitor.need_forward_ref = false;
            return;
          }
        }
      }
    }
  }

  // Generate a unique identifier for forwardRef
  let local = visitor.alloc_unique_ident_for("forwardRef", "__cmplf");
  let local_name = local.sym.to_string();
  let imported_name = if local_name == "forwardRef" {
    None
  } else {
    Some("forwardRef")
  };
  let specifier = create_import_specifier(&local_name, imported_name);

  // Try to add to existing react import
  if add_to_existing_import(m, "react", specifier.clone()) {
    visitor.forward_ref_ident = Some(local);
    visitor.need_forward_ref = false;
    return;
  }

  // Create new import if no existing react import found
  let new_import = create_import_decl("react", vec![specifier]);
  prepend_import(m, new_import);
  visitor.forward_ref_ident = Some(local);
  visitor.need_forward_ref = false;
}

fn ensure_runtime_import(m: &mut Module, import_name: &str) -> bool {
  const RUNTIME_MODULE: &str = "@compiled/react/runtime";

  // Check if already imported
  if find_existing_import_specifier(m, RUNTIME_MODULE, import_name) {
    return false; // Already exists, no need to add
  }

  // Try to add to existing @compiled/react/runtime import
  let specifier = create_import_specifier(import_name, None);
  if add_to_existing_import(m, RUNTIME_MODULE, specifier.clone()) {
    return true;
  }

  // Create new import
  let new_import = create_import_decl(RUNTIME_MODULE, vec![specifier]);
  prepend_import(m, new_import);
  true
}

pub fn ensure_runtime_ax_import(visitor: &mut AtomicCssCollector, m: &mut Module) {
  if !visitor.need_runtime_ax {
    return;
  }

  ensure_runtime_import(m, "ax");
  visitor.need_runtime_ax = false;
}

pub fn ensure_runtime_ix_import(visitor: &mut AtomicCssCollector, m: &mut Module) {
  if !visitor.need_runtime_ix {
    return;
  }

  ensure_runtime_import(m, "ix");
  visitor.need_runtime_ix = false;
}

pub fn ensure_runtime_cc_cs_imports(visitor: &mut AtomicCssCollector, m: &mut Module) {
  if !visitor.need_runtime_cc && !visitor.need_runtime_cs {
    return;
  }

  let mut specifiers = Vec::new();

  if visitor.need_runtime_cc && !find_existing_import_specifier(m, "@compiled/react/runtime", "CC")
  {
    specifiers.push(create_import_specifier("CC", None));
  }

  if visitor.need_runtime_cs && !find_existing_import_specifier(m, "@compiled/react/runtime", "CS")
  {
    specifiers.push(create_import_specifier("CS", None));
  }

  if !specifiers.is_empty() {
    // Try to add to existing import
    let mut added_to_existing = false;
    for item in &mut m.body {
      if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = item
        && &*import_decl.src.value == "@compiled/react/runtime"
      {
        import_decl.specifiers.extend(specifiers.clone());
        added_to_existing = true;
        break;
      }
    }

    // Create new import if needed
    if !added_to_existing {
      let new_import = create_import_decl("@compiled/react/runtime", specifiers);
      prepend_import(m, new_import);
    }
  }

  visitor.need_runtime_cc = false;
  visitor.need_runtime_cs = false;
}
