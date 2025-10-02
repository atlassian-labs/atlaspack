use swc_common::{DUMMY_SP, SyntaxContext};
use swc_ecma_ast::*;

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

pub fn ensure_forward_ref_import(visitor: &mut AtomicCssCollector, m: &mut Module) {
  if !visitor.need_forward_ref {
    return;
  }

  // Try to find existing react import and forwardRef specifier
  for item in &mut m.body {
    if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = item
      && &*import_decl.src.value == "react"
    {
      // search named specifiers
      for spec in &import_decl.specifiers {
        if let ImportSpecifier::Named(named) = spec {
          match &named.imported {
            Some(ModuleExportName::Ident(id)) if &*id.sym == "forwardRef" => {
              visitor.forward_ref_ident = Some(named.local.clone());
              visitor.used_idents.insert(named.local.sym.to_string());
              visitor.need_forward_ref = false;
              return;
            }
            Some(ModuleExportName::Str(s)) if &*s.value == "forwardRef" => {
              visitor.forward_ref_ident = Some(named.local.clone());
              visitor.used_idents.insert(named.local.sym.to_string());
              visitor.need_forward_ref = false;
              return;
            }
            _ => {}
          }
        }
      }
    }
  }

  // Try to extend first react import
  for item in &mut m.body {
    if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = item
      && &*import_decl.src.value == "react"
    {
      let local = visitor.alloc_unique_ident_for("forwardRef", "__cmplf");
      let imported_ident = Ident::new("forwardRef".into(), DUMMY_SP, SyntaxContext::empty());
      let imported = if &*local.sym == "forwardRef" {
        None
      } else {
        Some(ModuleExportName::Ident(imported_ident))
      };
      import_decl
        .specifiers
        .push(ImportSpecifier::Named(ImportNamedSpecifier {
          span: DUMMY_SP,
          local: local.clone(),
          imported,
          is_type_only: false,
        }));
      visitor.forward_ref_ident = Some(local);
      visitor.need_forward_ref = false;
      return;
    }
  }

  // Otherwise, insert a new import at the top
  let local = visitor.alloc_unique_ident_for("forwardRef", "__cmplf");
  let imported_ident = Ident::new("forwardRef".into(), DUMMY_SP, SyntaxContext::empty());
  let imported = if &*local.sym == "forwardRef" {
    None
  } else {
    Some(ModuleExportName::Ident(imported_ident))
  };
  let new_import = ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
      span: DUMMY_SP,
      local: local.clone(),
      imported,
      is_type_only: false,
    })],
    src: Box::new(Str {
      span: DUMMY_SP,
      value: "react".into(),
      raw: None,
    }),
    type_only: false,
    with: None,
    phase: Default::default(),
  }));
  let mut items = Vec::with_capacity(m.body.len() + 1);
  items.push(new_import);
  items.append(&mut m.body);
  m.body = items;
  visitor.forward_ref_ident = Some(local);
  visitor.need_forward_ref = false;
}

pub fn ensure_runtime_ax_import(visitor: &mut AtomicCssCollector, m: &mut Module) {
  if !visitor.need_runtime_ax {
    return;
  }

  // Try to find existing @compiled/react/runtime import and ax specifier
  for item in &mut m.body {
    if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = item
      && &*import_decl.src.value == "@compiled/react/runtime"
    {
      for spec in &import_decl.specifiers {
        if let ImportSpecifier::Named(named) = spec {
          match &named.imported {
            Some(ModuleExportName::Ident(id)) if &*id.sym == "ax" => {
              visitor.need_runtime_ax = false;
              return;
            }
            Some(ModuleExportName::Str(s)) if &*s.value == "ax" => {
              visitor.need_runtime_ax = false;
              return;
            }
            None => {
              if &*named.local.sym == "ax" {
                visitor.need_runtime_ax = false;
                return;
              }
            }
            _ => {}
          }
        }
      }
    }
  }

  // Try to extend first @compiled/react/runtime import
  for item in &mut m.body {
    if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = item
      && &*import_decl.src.value == "@compiled/react/runtime"
    {
      import_decl
        .specifiers
        .push(ImportSpecifier::Named(ImportNamedSpecifier {
          span: DUMMY_SP,
          local: Ident::new("ax".into(), DUMMY_SP, SyntaxContext::empty()),
          imported: Some(ModuleExportName::Ident(Ident::new(
            "ax".into(),
            DUMMY_SP,
            SyntaxContext::empty(),
          ))),
          is_type_only: false,
        }));
      visitor.need_runtime_ax = false;
      return;
    }
  }

  // Otherwise, insert new import at the top
  let new_import = ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
      span: DUMMY_SP,
      local: Ident::new("ax".into(), DUMMY_SP, SyntaxContext::empty()),
      imported: Some(ModuleExportName::Ident(Ident::new(
        "ax".into(),
        DUMMY_SP,
        SyntaxContext::empty(),
      ))),
      is_type_only: false,
    })],
    src: Box::new(Str {
      span: DUMMY_SP,
      value: "@compiled/react/runtime".into(),
      raw: None,
    }),
    type_only: false,
    with: None,
    phase: Default::default(),
  }));
  let mut items = Vec::with_capacity(m.body.len() + 1);
  items.push(new_import);
  items.append(&mut m.body);
  m.body = items;
  visitor.need_runtime_ax = false;
}

pub fn ensure_runtime_ix_import(visitor: &mut AtomicCssCollector, m: &mut Module) {
  if !visitor.need_runtime_ix {
    return;
  }

  // Try to find existing @compiled/react/runtime import and ix specifier
  for item in &mut m.body {
    if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = item
      && &*import_decl.src.value == "@compiled/react/runtime"
    {
      for spec in &import_decl.specifiers {
        if let ImportSpecifier::Named(named) = spec {
          match &named.imported {
            Some(ModuleExportName::Ident(id)) if &*id.sym == "ix" => {
              visitor.need_runtime_ix = false;
              return;
            }
            Some(ModuleExportName::Str(s)) if &*s.value == "ix" => {
              visitor.need_runtime_ix = false;
              return;
            }
            None => {
              if &*named.local.sym == "ix" {
                visitor.need_runtime_ix = false;
                return;
              }
            }
            _ => {}
          }
        }
      }
    }
  }

  // Try to extend first @compiled/react/runtime import
  for item in &mut m.body {
    if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = item
      && &*import_decl.src.value == "@compiled/react/runtime"
    {
      import_decl
        .specifiers
        .push(ImportSpecifier::Named(ImportNamedSpecifier {
          span: DUMMY_SP,
          local: Ident::new("ix".into(), DUMMY_SP, SyntaxContext::empty()),
          imported: Some(ModuleExportName::Ident(Ident::new(
            "ix".into(),
            DUMMY_SP,
            SyntaxContext::empty(),
          ))),
          is_type_only: false,
        }));
      visitor.need_runtime_ix = false;
      return;
    }
  }

  // Otherwise, insert new import at the top
  let new_import = ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
      span: DUMMY_SP,
      local: Ident::new("ix".into(), DUMMY_SP, SyntaxContext::empty()),
      imported: Some(ModuleExportName::Ident(Ident::new(
        "ix".into(),
        DUMMY_SP,
        SyntaxContext::empty(),
      ))),
      is_type_only: false,
    })],
    src: Box::new(Str {
      span: DUMMY_SP,
      value: "@compiled/react/runtime".into(),
      raw: None,
    }),
    type_only: false,
    with: None,
    phase: Default::default(),
  }));

  // let new_import_compiled_css_in_js = ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
  //   span: DUMMY_SP,
  //   specifiers: vec![],
  //   src: Box::new(Str {
  //     span: DUMMY_SP,
  //     value: "@atlaspack/compiled-css-in-js-stylesheet".into(),
  //     raw: None,
  //   }),
  //   type_only: false,
  //   with: None,
  //   phase: Default::default(),
  // }));

  let mut items = Vec::with_capacity(m.body.len() + 1);
  items.push(new_import);
  // items.push(new_import_compiled_css_in_js);
  items.append(&mut m.body);
  m.body = items;
  visitor.need_runtime_ix = false;
}
