use std::collections::HashSet;
use std::path::Path;

use atlaspack_swc_runner::runner::RunVisitResult;
use clap::Parser;
use rayon::iter::{ParallelBridge, ParallelIterator};
use swc_core::common::sync::Lrc;
use swc_core::common::FileName;
use swc_core::common::SourceMap;
use swc_core::ecma::parser::lexer::Lexer;
use swc_core::ecma::parser::StringInput;
use swc_core::ecma::visit::Visit;
use swc_core::ecma::visit::VisitMutWith;
use swc_core::ecma::visit::VisitWith;
use swc_core::{
  common::comments::{Comments, SingleThreadedComments},
  ecma::ast::SpanExt,
};

#[derive(Parser)]
struct Args {
  target: String,
}

fn main() -> anyhow::Result<()> {
  tracing_subscriber::fmt::init();

  let args = Args::parse();
  let walker = jwalk::WalkDir::new(args.target).into_iter();
  let extensions: HashSet<_> = ["tsx", "ts", "js", "jsx"].into_iter().collect();
  let start = std::time::Instant::now();

  let tasks: Vec<_> = walker
    .par_bridge()
    .map(|entry| -> anyhow::Result<Option<_>> {
      let entry = entry?;
      if !entry.path().is_file() {
        return Ok(None);
      }

      if !extensions.contains(
        &entry
          .path()
          .extension()
          .unwrap_or_default()
          .to_str()
          .unwrap(),
      ) {
        return Ok(None);
      }

      let file = std::fs::read_to_string(entry.path())?;
      let result = run_visit(&entry.path(), &file)?;

      Ok(Some(()))
    })
    .map(|task| {
      if let Err(err) = &task {
        tracing::error!("Error: {}", err);
      }

      task
    })
    .filter_map(|task| task.transpose())
    .collect();

  let elapsed = start.elapsed();
  tracing::info!(
    "processed {} files errors {} in {:?}",
    tasks.len(),
    tasks.iter().filter(|task| task.is_err()).count(),
    elapsed
  );

  for task in tasks {
    match task {
      Ok(visitor) => tracing::debug!("{:?}", visitor),
      Err(err) => tracing::debug!("Error: {}", err),
    }
  }

  Ok(())
}

fn run_visit(path: &Path, file: &str) -> anyhow::Result<RunVisitResult<DocsResult>> {
  let source_map = Lrc::new(SourceMap::default());
  let source_file = source_map.new_source_file(Lrc::new(FileName::Anon), file.into());

  let comments = SingleThreadedComments::default();
  let extension = path.extension().unwrap_or_default();
  let syntax = if extension == "tsx" {
    swc_core::ecma::parser::Syntax::Typescript(swc_core::ecma::parser::TsSyntax {
      tsx: true,
      ..Default::default()
    })
  } else if extension == "ts" {
    swc_core::ecma::parser::Syntax::Typescript(swc_core::ecma::parser::TsSyntax {
      ..Default::default()
    })
  } else if extension == "jsx" {
    swc_core::ecma::parser::Syntax::Es(swc_core::ecma::parser::EsSyntax {
      jsx: true,
      ..Default::default()
    })
  } else {
    swc_core::ecma::parser::Syntax::Es(swc_core::ecma::parser::EsSyntax {
      ..Default::default()
    })
  };

  let lexer = Lexer::new(
    syntax,
    Default::default(),
    StringInput::from(&*source_file),
    Some(&comments),
  );

  let mut parser = swc_core::ecma::parser::Parser::new_from(lexer);
  let mut program = parser
    .parse_program()
    .map_err(|err| anyhow::anyhow!("Error parsing {}: {:?}", path.display(), err))?;

  let (output_code, docs_result, output_map_buffer) = swc_core::common::GLOBALS.set(
    &swc_core::common::Globals::new(),
    move || -> anyhow::Result<_> {
      let global_mark = swc_core::common::Mark::new();
      let unresolved_mark = swc_core::common::Mark::new();

      program.visit_mut_with(&mut swc_core::ecma::transforms::base::resolver(
        unresolved_mark,
        global_mark,
        false,
      ));

      let context = atlaspack_swc_runner::runner::RunContext {
        is_module: program.is_module(),
        source_map: source_map.clone(),
        global_mark,
        unresolved_mark,
        comments: comments.clone(),
      };

      let mut docs_visitor = DocsVisitor::new(&context.comments);
      program.visit_with(&mut docs_visitor);

      let mut line_pos_buffer = vec![];
      let mut output_buffer = vec![];
      let writer = swc_core::ecma::codegen::text_writer::JsWriter::new(
        source_map.clone(),
        "\n",
        &mut output_buffer,
        Some(&mut line_pos_buffer),
      );

      let mut emitter = swc_core::ecma::codegen::Emitter {
        cfg: swc_core::ecma::codegen::Config::default(),
        cm: source_map.clone(),
        comments: None,
        wr: writer,
      };

      emitter.emit_program(&program)?;

      let output_code = String::from_utf8(output_buffer)?;
      let source_map = source_map.build_source_map(&line_pos_buffer);
      let mut output_map_buffer = vec![];

      source_map.to_writer(&mut output_map_buffer)?;

      Ok((output_code, docs_visitor.get_result(), output_map_buffer))
    },
  )?;

  // tracing::info!("processed {}", path.display());

  Ok(RunVisitResult {
    output_code,
    visitor: docs_result,
    source_map: output_map_buffer,
  })
}

struct DocsResult;

#[derive(Debug)]
struct DocsVisitor<'a> {
  comments: &'a SingleThreadedComments,
}

impl<'a> DocsVisitor<'a> {
  fn new(comments: &'a SingleThreadedComments) -> Self {
    Self { comments }
  }

  fn get_result(self) -> DocsResult {
    DocsResult
  }
}

impl<'a> Visit for DocsVisitor<'a> {
  fn visit_class_decl(&mut self, node: &swc_core::ecma::ast::ClassDecl) {
    let range = node.comment_range();
    let comments = self.comments.get_leading(range.lo());

    if let Some(comments) = comments {
      if comments[0].text.starts_with("*") {
        println!("{:?}", comments);
        println!("{:?}", node.ident);
      }
    }
  }

  fn visit_fn_decl(&mut self, node: &swc_core::ecma::ast::FnDecl) {
    let range = node.comment_range();
    let comments = self.comments.get_leading(range.lo());

    if let Some(comments) = comments {
      if comments[0].text.starts_with("*") {
        println!("{:?}", comments);
        println!("{:?}", node.ident);
      }
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_run_visit() {
    let file_name = "anon.tsx";
    let file = r#"
/**
 * Renders a friendly message.
 */
export function MyComponent() {
  return <div>Hello, world!</div>;
}
    "#;

    let result = run_visit(Path::new(file_name), file);
    let result = result.unwrap();
  }
}
