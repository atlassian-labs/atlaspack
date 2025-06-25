use swc_core::common::sync::Lrc;
use swc_core::common::SyntaxContext;
use swc_core::common::{SourceMap, Spanned, DUMMY_SP};
use swc_core::ecma::ast::*;
use swc_core::ecma::codegen::text_writer::JsWriter;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};

pub struct LoadableTypeReplacer {
  types_to_replace: Vec<String>,
}

impl LoadableTypeReplacer {
  pub fn new(types_to_replace: Vec<String>) -> Self {
    Self { types_to_replace }
  }

  /// Create a replacer for specific loadable types
  pub fn for_types(types: &[&str]) -> Self {
    Self {
      types_to_replace: types.iter().map(|s| s.to_string()).collect(),
    }
  }

  /// Check if an expression is a call to one of the loadable types we want to replace
  fn should_replace_loadable(&self, expr: &Expr) -> bool {
    let name = match expr {
      Expr::Ident(ident) => Some(ident.sym.to_string()),
      Expr::Member(member) => {
        if let MemberProp::Ident(prop) = &member.prop {
          Some(prop.sym.to_string())
        } else {
          None
        }
      }
      _ => None,
    };

    if let Some(name) = name {
      self.types_to_replace.contains(&name)
    } else {
      false
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::make_test_swc_config;
  use swc_core::ecma::visit::VisitMutWith;

  #[test]
  fn test_loadable_type_replacer() {
    let code = r#"
            import { LoadableBackground, LoadableLazy } from '@confluence/loadable';

            export const PageTreeLoaderBackground = LoadableBackground({
                loader: async () =>
                    (await import(
                        /* webpackChunkName: "loadable-PageTree" */ './PageTree'
                    )).PageTree,
            });

            export const InlineCommentsLoader = LoadableBackground({
                loader: async () =>
                    (await import(
                        /* webpackChunkName: "loadable-inline-comments" */ '../InlineComments'
                    )).InlineComments,
            });

            export const LazyComponent = LoadableLazy({
                loader: () =>
                    import(/* webpackChunkName: "loadable-lazy" */ './LazyComponent'),
            });

            // This one should not be replaced
            export const PaintComponent = LoadablePaint({
                loader: () =>
                    import(/* webpackChunkName: "loadable-paint" */ './PaintComponent'),
            });
        "#;

    let mut config = make_test_swc_config(code);
    config.is_type_script = true;
    config.is_jsx = true;
    let source_map = Lrc::new(SourceMap::default());
    let (mut program, _) = crate::parse(
      code,
      &config.project_root,
      &config.filename,
      &source_map,
      &config,
    )
    .unwrap();

    let mut replacer = LoadableTypeReplacer::for_types(&["LoadableBackground", "LoadableLazy"]);
    program.visit_mut_with(&mut replacer);

    // Convert back to string to verify
    let mut buf = vec![];
    let writer = JsWriter::new(source_map.clone(), "\n", &mut buf, None);
    let config = swc_core::ecma::codegen::Config::default();
    let mut emitter = swc_core::ecma::codegen::Emitter {
      cfg: config,
      comments: None,
      cm: source_map.clone(),
      wr: writer,
    };
    emitter.emit_program(&program).unwrap();
    let output = String::from_utf8(buf).unwrap();

    // Print the actual output for debugging
    println!("Generated output:\n{}", output);

    // Verify the output contains empty loader functions for both types
    assert!(output.contains("LoadableBackground({"));
    assert!(output.contains("LoadableLazy({"));
    assert!(output.contains("LoadablePaint({")); // This one should remain unchanged

    // Verify the Background loaders were replaced
    assert!(output.contains("PageTreeLoaderBackground = LoadableBackground({"));
    assert!(output.contains("InlineCommentsLoader = LoadableBackground({"));

    // Verify the Lazy loader was replaced
    assert!(output.contains("LazyComponent = LoadableLazy({"));

    // Verify the Paint loader was not replaced
    assert!(output.contains("PaintComponent = LoadablePaint({"));
    assert!(output.contains("import('./PaintComponent')"));

    // Verify the imports were properly handled
    assert!(!output.contains("./PageTree"));
    assert!(!output.contains("../InlineComments"));
    assert!(!output.contains("./LazyComponent"));
  }
}

impl VisitMut for LoadableTypeReplacer {
  fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
    // First check if this is a loadable function call we want to replace
    if let Callee::Expr(expr) = &node.callee {
      if self.should_replace_loadable(&**expr) {
        // If it is, look for the loader property in its argument
        if let Some(arg) = node.args.first_mut() {
          if let Expr::Object(obj) = &mut *arg.expr {
            // Find the loader property
            for prop in &mut obj.props {
              if let PropOrSpread::Prop(prop) = prop {
                if let Prop::KeyValue(kv) = &mut **prop {
                  if let PropName::Ident(ident) = &kv.key {
                    if ident.sym == *"loader" {
                      // Replace the loader function with () => {}
                      kv.value = Box::new(Expr::Arrow(ArrowExpr {
                        span: DUMMY_SP,
                        params: vec![],
                        body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
                          span: DUMMY_SP,
                          stmts: vec![],
                          ctxt: SyntaxContext::empty(),
                        })),
                        is_async: false,
                        is_generator: false,
                        type_params: None,
                        return_type: None,
                        ctxt: SyntaxContext::empty(),
                      }));
                    }
                  }
                }
              }
            }
          }
        }
      }
    }

    // Continue visiting children
    node.visit_mut_children_with(self);
  }
}
