use swc_core::ecma::ast::*;
use swc_core::ecma::visit::VisitMut;
use swc_core::quote;

use self::react_detector::{
  arrow_contains_component, function_contains_component, is_component_name,
};

mod react_detector;

/// SWC visitor that annotates top-level React components with a
/// `displayName` property to ease debugging and profiling.
///
/// A declaration is considered a component when:
///
/// 1. the identifier starts with an upper-case character **and**
/// 2. its body contains either JSX **or** a React Hook call (`useXxx`).
///
/// # Example
///
/// ```text
/// export function Foo() { return <div />; }
/// ```
/// becomes
/// ```text
/// export function Foo() { return <div />; }
/// Foo.displayName = "Foo";
/// ```
///
/// Usage inside another SWC pass:
/// ```rust,no_run
/// use swc_core::ecma::visit::VisitMutWith;
/// # let mut module: swc_core::ecma::ast::Module = todo!();
/// module.visit_mut_with(&mut atlaspack_js_swc_core::AddDisplayNameVisitor::default());
/// ```
#[derive(Default)]
pub struct AddDisplayNameVisitor {}

impl VisitMut for AddDisplayNameVisitor {
  fn visit_mut_module(&mut self, module: &mut Module) {
    let mut new_body = Vec::with_capacity(module.body.len());

    for item in std::mem::take(&mut module.body) {
      match &item {
        ModuleItem::Stmt(Stmt::Decl(decl)) => {
          if let Some(assign) = build_assignment_for_decl(decl) {
            new_body.push(item);
            new_body.push(assign);
            continue;
          }
        }
        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => {
          if let Some(assign) = build_assignment_for_decl(&export.decl) {
            new_body.push(item);
            new_body.push(assign);
            continue;
          }
        }
        ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(default_decl)) => {
          if let Some(assign) = build_assignment_for_default(default_decl) {
            new_body.push(item);
            new_body.push(assign);
            continue;
          }
        }
        _ => {}
      }
      new_body.push(item);
    }

    module.body = new_body;
  }
}

fn build_assignment_for_decl(decl: &Decl) -> Option<ModuleItem> {
  match decl {
    Decl::Fn(fn_decl) => {
      let ident = &fn_decl.ident;
      if is_component_name(ident) && function_contains_component(&fn_decl.function) {
        return Some(create_display_name_assignment(ident));
      }
    }
    Decl::Class(class_decl) => {
      let ident = &class_decl.ident;
      if is_component_name(ident) {
        return Some(create_display_name_assignment(ident));
      }
    }
    Decl::Var(var_decl) => {
      for declarator in &var_decl.decls {
        if let Pat::Ident(binding) = &declarator.name {
          let ident = &binding.id;
          if !is_component_name(ident) {
            continue;
          }
          if let Some(init) = &declarator.init {
            match &**init {
              Expr::Fn(fn_expr) if function_contains_component(&fn_expr.function) => {
                return Some(create_display_name_assignment(ident));
              }
              Expr::Arrow(arrow) if arrow_contains_component(arrow) => {
                return Some(create_display_name_assignment(ident));
              }
              _ => {}
            }
          }
        }
      }
    }
    _ => {}
  }
  None
}

fn build_assignment_for_default(decl: &ExportDefaultDecl) -> Option<ModuleItem> {
  match &decl.decl {
    DefaultDecl::Fn(fn_expr) => {
      if let Some(id) = &fn_expr.ident {
        if is_component_name(id) && function_contains_component(&fn_expr.function) {
          return Some(create_display_name_assignment(id));
        }
      }
    }
    DefaultDecl::Class(class_expr) => {
      if let Some(id) = &class_expr.ident {
        if is_component_name(id) {
          return Some(create_display_name_assignment(id));
        }
      }
    }
    _ => {}
  }
  None
}

fn create_display_name_assignment(id: &Ident) -> ModuleItem {
  let comp = id.clone();
  let name = Expr::Lit(Lit::Str(Str {
    span: id.span,
    value: id.sym.clone(),
    raw: None,
  }));

  quote!(
    "$comp.displayName = $name_str;" as ModuleItem,
    comp: Ident = comp,
    name_str: Expr = name,
  )
}

#[cfg(test)]
mod tests {
  use atlaspack_swc_runner::runner::{
    run_with_transformation, RunWithTransformationOptions, RunWithTransformationOutput,
  };
  use pretty_assertions::assert_str_eq;
  use swc_ecma_parser::{EsSyntax, Syntax};

  use super::*;

  fn transform(code: &str) -> String {
    let options = RunWithTransformationOptions {
      code,
      syntax: Some(Syntax::Es(EsSyntax {
        jsx: true,
        ..Default::default()
      })),
    };

    let RunWithTransformationOutput { output_code, .. } =
      run_with_transformation(options, |_ctx, module| {
        let mut visitor = AddDisplayNameVisitor::default();
        visitor.visit_mut_module(module);
        visitor
      })
      .unwrap();

    output_code
  }

  #[test]
  fn test_fn_with_jsx() {
    let out = transform("export function Foo() { return <div />; }");
    assert_str_eq!(
      out.trim(),
      "
export function Foo() {
    return <div/>;
}
Foo.displayName = \"Foo\";"
        .trim()
    );
  }

  #[test]
  fn test_arrow_with_jsx() {
    let out = transform("export const Bar = () => <div />;");
    assert_str_eq!(
      out.trim(),
      "
export const Bar = ()=><div/>;
Bar.displayName = \"Bar\";"
        .trim()
    );
  }

  #[test]
  fn test_fn_with_hook() {
    let out = transform("export function Comp() { useEffect(() => {}, []); return null; }");
    assert_str_eq!(
      out.trim(),
      "
export function Comp() {
    useEffect(()=>{}, []);
    return null;
}
Comp.displayName = \"Comp\";"
        .trim()
    );
  }

  #[test]
  fn test_non_component() {
    let out = transform("export function NotComp() { return 1; }");
    assert_str_eq!(
      out.trim(),
      "
export function NotComp() {
    return 1;
}"
      .trim()
    );
  }

  #[test]
  fn test_default_export_named_function() {
    let out = transform("export default function Foo() { return <div />; }");
    assert_str_eq!(
      out.trim(),
      "
export default function Foo() {
    return <div/>;
}
Foo.displayName = \"Foo\";"
        .trim()
    );
  }

  #[test]
  fn test_default_export_named_class() {
    let out = transform("export default class Foo {}");
    assert_str_eq!(
      out.trim(),
      "
export default class Foo {
}
Foo.displayName = \"Foo\";"
        .trim()
    );
  }

  #[test]
  fn test_non_exported_component_function() {
    let out = transform("function Local() { return <div />; }");
    assert_str_eq!(
      out.trim(),
      "
function Local() {
    return <div/>;
}
Local.displayName = \"Local\";"
        .trim()
    );
  }
}
