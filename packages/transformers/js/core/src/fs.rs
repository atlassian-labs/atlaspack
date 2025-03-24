use std::path::Path;
use std::path::PathBuf;

use data_encoding::BASE64;
use data_encoding::HEXLOWER;
use swc_core::common::Mark;
use swc_core::common::Span;
use swc_core::common::SyntaxContext;
use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::*;
use swc_core::ecma::atoms::JsWord;
use swc_core::ecma::utils::stack_size::maybe_grow_default;
use swc_core::ecma::visit::Fold;
use swc_core::ecma::visit::FoldWith;
use swc_core::ecma::visit::VisitWith;

use crate::collect::Collect;
use crate::collect::Import;
use crate::dependency_collector::DependencyDescriptor;
use crate::dependency_collector::DependencyKind;
use crate::id;
use crate::utils::SourceLocation;

#[allow(clippy::too_many_arguments)]
pub fn inline_fs<'a>(
  filename: &str,
  source_map: swc_core::common::sync::Lrc<swc_core::common::SourceMap>,
  unresolved_mark: Mark,
  global_mark: Mark,
  project_root: &'a str,
  deps: &'a mut Vec<DependencyDescriptor>,
  is_module: bool,
  conditional_bundling: bool,
) -> impl Fold + 'a {
  InlineFS {
    filename: Path::new(filename).to_path_buf(),
    collect: Collect::new(
      source_map,
      unresolved_mark,
      Mark::fresh(Mark::root()),
      global_mark,
      false,
      is_module,
      conditional_bundling,
    ),
    project_root,
    deps,
  }
}

struct InlineFS<'a> {
  filename: PathBuf,
  collect: Collect,
  project_root: &'a str,
  deps: &'a mut Vec<DependencyDescriptor>,
}

impl Fold for InlineFS<'_> {
  fn fold_module(&mut self, node: Module) -> Module {
    node.visit_with(&mut self.collect);
    node.fold_children_with(self)
  }

  fn fold_expr(&mut self, node: Expr) -> Expr {
    if let Expr::Call(call) = &node {
      if let Callee::Expr(expr) = &call.callee {
        if let Some((source, specifier)) = self.match_module_reference(expr) {
          if &source == "fs" && &specifier == "readFileSync" {
            if let Some(arg) = call.args.first() {
              if let Some(res) = self.evaluate_fs_arg(&arg.expr, call.args.get(1), call.span) {
                return res;
              }
            }
          }
        }
      }
    }

    maybe_grow_default(|| node.fold_children_with(self))
  }
}

impl InlineFS<'_> {
  fn match_module_reference(&self, node: &Expr) -> Option<(JsWord, JsWord)> {
    match node {
      Expr::Ident(ident) => {
        if let Some(Import {
          source, specifier, ..
        }) = self.collect.imports.get(&id!(ident))
        {
          return Some((source.clone(), specifier.clone()));
        }
      }
      Expr::Member(member) => {
        let prop = match &member.prop {
          MemberProp::Ident(ident) => ident.sym.clone(),
          MemberProp::Computed(ComputedPropName { expr, .. }) => {
            if let Expr::Lit(Lit::Str(str_)) = &**expr {
              str_.value.clone()
            } else {
              return None;
            }
          }
          _ => return None,
        };

        if let Some(source) = self.collect.match_require(&member.obj) {
          return Some((source, prop));
        }

        if let Expr::Ident(ident) = &*member.obj {
          if let Some(Import {
            source, specifier, ..
          }) = self.collect.imports.get(&id!(ident))
          {
            if specifier == "default" || specifier == "*" {
              return Some((source.clone(), prop));
            }
          }
        }
      }
      _ => {}
    }

    None
  }

  fn evaluate_fs_arg(
    &mut self,
    node: &Expr,
    encoding: Option<&ExprOrSpread>,
    span: Span,
  ) -> Option<Expr> {
    let mut evaluator = Evaluator { inline: self };

    let res = node.clone().fold_with(&mut evaluator);
    match res {
      Expr::Lit(Lit::Str(str_)) => {
        // Ignore if outside the project root
        let path = match dunce::canonicalize(Path::new(&str_.value.to_string())) {
          Ok(path) => path,
          Err(_err) => return None,
        };
        if !path.starts_with(self.project_root) {
          return None;
        }

        let encoding = match encoding {
          Some(e) => match &*e.expr {
            Expr::Lit(Lit::Str(str_)) => &str_.value,
            _ => "buffer",
          },
          None => "buffer",
        };

        // TODO: this should probably happen in JS so we use Atlaspack's file system
        // rather than only the real FS. Will need when we convert to WASM.
        let contents = match encoding {
          "base64" | "buffer" => {
            if let Ok(contents) = std::fs::read(&path) {
              BASE64.encode(&contents)
            } else {
              return None;
            }
          }
          "hex" => {
            if let Ok(contents) = std::fs::read(&path) {
              HEXLOWER.encode(&contents)
            } else {
              return None;
            }
          }
          "utf8" | "utf-8" => {
            if let Ok(contents) = std::fs::read_to_string(&path) {
              contents
            } else {
              return None;
            }
          }
          _ => return None,
        };

        let contents = Expr::Lit(Lit::Str(contents.into()));

        // Add a file dependency so the cache is invalidated when this file changes.
        self.deps.push(DependencyDescriptor {
          kind: DependencyKind::File,
          loc: SourceLocation::from(&self.collect.source_map, span),
          specifier: path.to_str().unwrap().into(),
          attributes: None,
          is_optional: false,
          is_helper: false,
          source_type: None,
          placeholder: None,
        });

        // If buffer, wrap in Buffer.from(base64String, 'base64')
        if encoding == "buffer" {
          Some(Expr::Call(CallExpr {
            callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
              obj: Box::new(Expr::Ident(Ident::new(
                "Buffer".into(),
                DUMMY_SP,
                SyntaxContext::empty().apply_mark(self.collect.unresolved_mark),
              ))),
              prop: MemberProp::Ident(IdentName::new("from".into(), DUMMY_SP)),
              span: DUMMY_SP,
            }))),
            args: vec![
              ExprOrSpread {
                expr: Box::new(contents),
                spread: None,
              },
              ExprOrSpread {
                expr: Box::new(Expr::Lit(Lit::Str("base64".into()))),
                spread: None,
              },
            ],
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            type_args: None,
          }))
        } else {
          Some(contents)
        }
      }
      _ => None,
    }
  }
}

struct Evaluator<'a> {
  inline: &'a InlineFS<'a>,
}

impl Fold for Evaluator<'_> {
  fn fold_expr(&mut self, node: Expr) -> Expr {
    let node = maybe_grow_default(|| node.fold_children_with(self));

    match &node {
      Expr::Ident(ident) => match ident.sym.to_string().as_str() {
        "__dirname" => Expr::Lit(Lit::Str(
          self
            .inline
            .filename
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .into(),
        )),
        "__filename" => Expr::Lit(Lit::Str(self.inline.filename.to_str().unwrap().into())),
        _ => node,
      },
      Expr::Bin(bin) => match bin.op {
        BinaryOp::Add => {
          let left = match &*bin.left {
            Expr::Lit(Lit::Str(str_)) => str_.value.clone(),
            _ => return node,
          };

          let right = match &*bin.right {
            Expr::Lit(Lit::Str(str_)) => str_.value.clone(),
            _ => return node,
          };

          Expr::Lit(Lit::Str(format!("{}{}", left, right).into()))
        }
        _ => node,
      },
      Expr::Call(call) => {
        let callee = match &call.callee {
          Callee::Expr(expr) => expr,
          _ => return node,
        };

        if let Some((source, specifier)) = self.inline.match_module_reference(callee) {
          match (source.to_string().as_str(), specifier.to_string().as_str()) {
            ("path", "join") => {
              let mut path = PathBuf::new();
              for arg in call.args.clone() {
                let s = match &*arg.expr {
                  Expr::Lit(Lit::Str(str_)) => str_.value.clone(),
                  _ => return node,
                };
                if path.as_os_str().is_empty() {
                  path.push(s.to_string());
                } else {
                  let s = s.to_string();
                  let mut p = Path::new(s.as_str());

                  // Node's path.join ignores separators at the start of path components.
                  // Rust's does not, so we need to strip them.
                  if let Ok(stripped) = p.strip_prefix("/") {
                    p = stripped;
                  }
                  path.push(p);
                }
              }

              return Expr::Lit(Lit::Str(path.to_str().unwrap().into()));
            }
            _ => return node,
          }
        }

        node
      }
      _ => node,
    }
  }
}

#[cfg(test)]
mod test {
  use atlaspack_swc_runner::{runner::RunVisitResult, test_utils::run_test_fold};

  use super::*;

  #[test]
  fn test_inline_fs_referencing_a_file_that_exists() {
    // Create a temporary directory
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_dir_path = std::fs::canonicalize(temp_dir.path()).unwrap();

    // Create a javascript file with code in temporary directory
    let sample_code = r#"
import fs from "fs";
import path from "path";

const content = fs.readFileSync(path.join(__dirname, "inline.txt"), "utf8");
    "#;
    std::fs::write(temp_dir_path.join("index.js"), sample_code).unwrap();
    // Create a txt file
    std::fs::write(temp_dir_path.join("inline.txt"), "Hello, world!").unwrap();

    let mut deps = vec![];
    let RunVisitResult { output_code, .. } = run_test_fold(sample_code, |ctx| {
      inline_fs(
        temp_dir_path.join("index.js").to_str().unwrap(),
        ctx.source_map,
        ctx.global_mark,
        ctx.global_mark,
        temp_dir_path.to_str().unwrap(),
        &mut deps,
        true,
        false,
      )
    });

    assert_eq!(
      output_code.trim(),
      r#"
import fs from "fs";
import path from "path";
const content = "Hello, world!";
      "#
      .trim()
    );
  }

  #[test]
  fn test_inline_fs_referencing_a_file_that_does_not_exist() {
    // Create a temporary directory
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_dir_path = std::fs::canonicalize(temp_dir.path()).unwrap();

    // Create a javascript file with code in temporary directory
    let sample_code = r#"
import fs from "fs";
import path from "path";

const content = fs.readFileSync(path.join(__dirname, "inline.txt"), "utf8");
    "#;
    std::fs::write(temp_dir_path.join("index.js"), sample_code).unwrap();

    let mut deps = vec![];
    let RunVisitResult { output_code, .. } = run_test_fold(sample_code, |ctx| {
      inline_fs(
        temp_dir_path.join("index.js").to_str().unwrap(),
        ctx.source_map,
        ctx.global_mark,
        ctx.global_mark,
        temp_dir_path.to_str().unwrap(),
        &mut deps,
        true,
        false,
      )
    });

    assert_eq!(
      output_code.trim(),
      r#"
import fs from "fs";
import path from "path";
const content = fs.readFileSync(path.join(__dirname, "inline.txt"), "utf8");
      "#
      .trim()
    );
  }

  #[test]
  fn test_inline_fs_with_a_file_path_string_concatenation() {
    // Create a temporary directory
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_dir_path = std::fs::canonicalize(temp_dir.path()).unwrap();

    // Create a javascript file with code in temporary directory
    let sample_code = r#"
import fs from "fs";

const content = fs.readFileSync(__dirname + "/inline.txt", "utf8");
    "#;
    std::fs::write(temp_dir_path.join("index.js"), sample_code).unwrap();
    // Create a txt file
    std::fs::write(temp_dir_path.join("inline.txt"), "Hello, world!").unwrap();

    let mut deps = vec![];
    let RunVisitResult { output_code, .. } = run_test_fold(sample_code, |ctx| {
      inline_fs(
        temp_dir_path.join("index.js").to_str().unwrap(),
        ctx.source_map,
        ctx.global_mark,
        ctx.global_mark,
        temp_dir_path.to_str().unwrap(),
        &mut deps,
        true,
        false,
      )
    });

    assert_eq!(
      output_code.trim(),
      r#"
import fs from "fs";
const content = "Hello, world!";
      "#
      .trim()
    );
  }
}
