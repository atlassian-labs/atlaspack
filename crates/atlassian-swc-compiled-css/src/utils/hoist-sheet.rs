use swc_core::atoms::Atom;
use swc_core::common::{SyntaxContext, DUMMY_SP};
use swc_core::ecma::ast::Ident;

use crate::postcss::plugins::extract_stylesheets::normalize_block_value_spacing;
use crate::types::Metadata;

fn next_identifier_name(counter: usize) -> String {
  if counter == 1 {
    "_".to_string()
  } else {
    format!("_{}", counter)
  }
}

/// Hoist a stylesheet string to the module scope and return the referencing
/// identifier, mirroring the behaviour of the Babel helper.
///
/// The SWC port tracks hoisted sheets on the shared transform state so the
/// enclosing visitor can emit the variable declarations at program scope.
pub fn hoist_sheet(sheet: &str, meta: &Metadata) -> Ident {
  let normalized = normalize_block_value_spacing(sheet);

  if let Some(existing) = meta.state().sheets.get(&normalized) {
    return existing.clone();
  }

  let mut state = meta.state_mut();

  if let Some(existing) = state.sheets.get(&normalized) {
    return existing.clone();
  }

  state.sheet_identifier_counter += 1;
  let name = next_identifier_name(state.sheet_identifier_counter);
  let ident = Ident::new(Atom::from(name), DUMMY_SP, SyntaxContext::empty());

  state.sheets.insert(normalized, ident.clone());

  ident
}

#[cfg(test)]
mod tests {
  use super::hoist_sheet;
  use swc_core::common::sync::Lrc;
  use swc_core::common::SourceMap;

  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};

  fn create_metadata() -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::new(cm, Vec::new());
    let state = std::rc::Rc::new(std::cell::RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));

    Metadata::new(state)
  }

  #[test]
  fn reuses_existing_identifier_for_sheet() {
    let meta = create_metadata();
    let sheet = "._1wyb1fwx{font-size:12px}";

    let first = hoist_sheet(sheet, &meta);
    let second = hoist_sheet(sheet, &meta);

    assert_eq!(first.sym, second.sym);

    let state = meta.state();
    assert_eq!(state.sheets.len(), 1);
    assert!(state.sheets.contains_key(sheet));
  }

  #[test]
  fn increments_identifier_suffix_for_new_sheets() {
    let meta = create_metadata();

    let first = hoist_sheet("._a{color:red}", &meta);
    assert_eq!(first.sym.as_ref(), "_");

    let second = hoist_sheet("._b{color:blue}", &meta);
    assert_eq!(second.sym.as_ref(), "_2");

    let third = hoist_sheet("._c{color:green}", &meta);
    assert_eq!(third.sym.as_ref(), "_3");
  }
}
