use std::path::PathBuf;

use swc_core::common::comments::SingleThreadedComments;
use swc_core::common::sync::Lrc;
use swc_core::common::*;
use swc_core::ecma::ast::*;
use swc_core::ecma::parser::lexer::Lexer;
use swc_core::ecma::parser::*;
use swc_core::ecma::visit::VisitWith;

use super::MagicCommentsVisitor;

pub fn parse(code: &str) -> anyhow::Result<Program> {
  let source_map = Lrc::new(SourceMap::default());
  let source_file =
    source_map.new_source_file(Lrc::new(FileName::Real(PathBuf::new())), code.into());

  let comments = SingleThreadedComments::default();
  let syntax = {
    let mut tsconfig = TsSyntax::default();
    tsconfig.tsx = true;
    Syntax::Typescript(tsconfig)
  };

  let lexer = Lexer::new(
    syntax,
    EsVersion::latest(),
    StringInput::from(&*source_file),
    Some(&comments),
  );

  let mut parser = Parser::new_from(lexer);

  let program = match parser.parse_program() {
    Err(err) => anyhow::bail!("{:?}", err),
    Ok(program) => program,
  };

  Ok(program)
}

#[test]
fn it_should_not_set_chunk_name_if_code_does_not_contain_a_magic_comment() -> anyhow::Result<()> {
  let code = r#"import('./foo')"#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert!(
    visitor.magic_comments.len() == 0,
    "Expected no magic comments to be set"
  );

  Ok(())
}

#[test]
fn it_should_set_chunk_name_if_code_contains_magic_comment() -> anyhow::Result<()> {
  let code = r#"import(/* webpackChunkName: "foo-chunk" */ "./foo")"#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert!(
    visitor.magic_comments.len() == 1,
    "Expected magic comment to be set"
  );

  let Some(chunk_name) = visitor.magic_comments.get("./foo") else {
    assert!(false, "Expected magic comment to be set");
    anyhow::bail!("")
  };

  assert!(
    chunk_name == "foo-chunk",
    "Expected magic comment to be set"
  );

  Ok(())
}

#[test]
fn it_should_set_chunk_name_if_code_contains_multiple_magic_comment() -> anyhow::Result<()> {
  let code = r#"
    import(/* webpackChunkName: "foo-chunk" */ "./foo")
    import(/* webpackChunkName: "bar-chunk" */ "./bar")
  "#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert!(
    visitor.magic_comments.len() == 2,
    "Expected magic comment to be set"
  );

  let Some(chunk_name_foo) = visitor.magic_comments.get("./foo") else {
    assert!(false, "Expected magic comment to be set");
    anyhow::bail!("")
  };

  let Some(chunk_name_bar) = visitor.magic_comments.get("./bar") else {
    assert!(false, "Expected magic comment to be set");
    anyhow::bail!("")
  };

  assert!(
    chunk_name_foo == "foo-chunk",
    "Expected magic comment to be set"
  );

  assert!(
    chunk_name_bar == "bar-chunk",
    "Expected magic comment to be set"
  );

  Ok(())
}

#[test]
fn it_should_work_with_current_dir_import() -> anyhow::Result<()> {
  let code = r#"import(/* webpackChunkName: "foo-chunk" */ ".");"#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert!(
    visitor.magic_comments.len() == 1,
    "Expected magic comment to be set"
  );

  let Some(chunk_name) = visitor.magic_comments.get(".") else {
    assert!(false, "Expected magic comment to be set");
    anyhow::bail!("")
  };

  assert!(
    chunk_name == "foo-chunk",
    "Expected magic comment to be set"
  );

  Ok(())
}

#[test]
fn it_should_work_with_current_dir_import_2() -> anyhow::Result<()> {
  let code = r#"import(/* webpackChunkName: "foo-chunk" */ "./");"#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert!(
    visitor.magic_comments.len() == 1,
    "Expected magic comment to be set"
  );

  let Some(chunk_name) = visitor.magic_comments.get("./") else {
    assert!(false, "Expected magic comment to be set");
    anyhow::bail!("")
  };

  assert!(
    chunk_name == "foo-chunk",
    "Expected magic comment to be set"
  );

  Ok(())
}

#[test]
fn it_should_work_with_parent_dir_import() -> anyhow::Result<()> {
  let code = r#"import(/* webpackChunkName: "foo-chunk" */ "..");"#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert!(
    visitor.magic_comments.len() == 1,
    "Expected magic comment to be set"
  );

  let Some(chunk_name) = visitor.magic_comments.get("..") else {
    assert!(false, "Expected magic comment to be set");
    anyhow::bail!("")
  };

  assert!(
    chunk_name == "foo-chunk",
    "Expected magic comment to be set"
  );

  Ok(())
}

#[test]
fn it_should_work_with_parent_dir_import_2() -> anyhow::Result<()> {
  let code = r#"import(/* webpackChunkName: "foo-chunk" */ "../");"#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert!(
    visitor.magic_comments.len() == 1,
    "Expected magic comment to be set"
  );

  let Some(chunk_name) = visitor.magic_comments.get("../") else {
    assert!(false, "Expected magic comment to be set");
    anyhow::bail!("")
  };

  assert!(
    chunk_name == "foo-chunk",
    "Expected magic comment to be set"
  );

  Ok(())
}

#[test]
fn it_parses_lazy_imports() -> anyhow::Result<()> {
  let code = r#"
    const LazyPermalinkButton: ComponentType<Props> = lazyAfterPaint(
      () =>
        import(/* webpackChunkName: "async-issue-view-permalink-button" */ './view').then(
          (exportedModule) => exportedModule.PermalinkButton,
        ),
        {
          ssr: false,
        },
    );
  "#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert!(
    visitor.magic_comments.len() == 1,
    "Expected magic comment to be set"
  );

  let Some(chunk_name) = visitor.magic_comments.get("./view") else {
    assert!(false, "Expected magic comment to be set");
    anyhow::bail!("")
  };

  assert!(
    chunk_name == "async-issue-view-permalink-button",
    "Expected magic comment to be set"
  );

  Ok(())
}
