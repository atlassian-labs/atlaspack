use std::collections::HashSet;

use swc_core::{
  atoms::Atom,
  ecma::{
    ast::{Pass, Pat::Ident, VarDecl},
    visit::{VisitMut, visit_mut_pass},
  },
};

pub fn declare_const_collector(declare_consts: &mut HashSet<Atom>) -> impl Pass + VisitMut {
  visit_mut_pass(DeclareConstCollector { declare_consts })
}

pub struct DeclareConstCollector<'a> {
  declare_consts: &'a mut HashSet<Atom>,
}

impl<'a> VisitMut for DeclareConstCollector<'a> {
  fn visit_mut_var_decl(&mut self, node: &mut VarDecl) {
    if node.declare {
      for declarator in &mut node.decls {
        if let Ident(binding) = &declarator.name {
          self.declare_consts.insert(binding.id.sym.clone());
        }
      }
    }
  }
}
