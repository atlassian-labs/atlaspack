use swc_core::common::comments::{Comment, CommentKind};
use swc_core::common::{BytePos, SourceMap, Span};

use crate::types::Metadata;

/// Represents the comments that appear immediately before and on the same line
/// as a node.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct NodeComments {
  pub before: Vec<Comment>,
  pub current: Vec<Comment>,
}

fn lookup_line(pos: BytePos, source_map: &SourceMap) -> Option<u32> {
  // `lookup_char_pos` will panic if the byte offset falls inside the middle of a multi-byte
  // character. This can happen if an upstream span is ill-formed (e.g. constructed using
  // character counts instead of byte offsets). Mirror Babel's tolerant behaviour by dropping
  // line info when a span is unusable rather than crashing the transform.
  let file = source_map.lookup_source_file(pos);
  let src = file.src.as_ref();

  if pos.0 < file.start_pos.0 {
    return None;
  }

  let rel = (pos.0 - file.start_pos.0) as usize;
  if rel > src.len() || !src.is_char_boundary(rel) {
    return None;
  }

  let line = src[..rel].bytes().filter(|b| *b == b'\n').count() as u32;
  Some(line + 1)
}

fn single_line(span: Span, source_map: &SourceMap) -> Option<u32> {
  let start_line = lookup_line(span.lo(), source_map)?;
  let end_pos = if span.hi() > span.lo() {
    BytePos(span.hi().0.saturating_sub(1))
  } else {
    span.hi()
  };
  let end_line = lookup_line(end_pos, source_map)?;

  if start_line == end_line {
    Some(start_line)
  } else {
    None
  }
}

/// Return the line and inline comments attached to the provided node span.
///
/// This mirrors the behaviour of `getNodeComments()` from the Babel
/// implementation by filtering the program comment list for line comments that
/// sit on the previous or current line of the node.
pub fn get_node_comments(span: Span, meta: &Metadata) -> NodeComments {
  let state = meta.state();
  let file = state.file();
  let source_map = file.source_map.clone();
  let comments = file.comments.clone();
  drop(state);

  let node_line = match single_line(span, &source_map) {
    Some(line) => line,
    None => return NodeComments::default(),
  };
  let previous_line = node_line.saturating_sub(1);

  let mut before = Vec::new();
  let mut current = Vec::new();

  for comment in comments.into_iter().filter(|c| c.kind == CommentKind::Line) {
    let Some(comment_line) = single_line(comment.span, &source_map) else {
      continue;
    };

    if comment_line == node_line {
      current.push(comment);
    } else if comment_line == previous_line {
      before.push(comment);
    }
  }

  NodeComments { before, current }
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;
  use std::rc::Rc;

  use swc_core::common::comments::{Comment, CommentKind};
  use swc_core::common::sync::Lrc;
  use swc_core::common::{BytePos, FileName, SourceFile, SourceMap, Span};

  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};

  use super::get_node_comments;

  fn build_metadata(cm: Lrc<SourceMap>, comments: Vec<Comment>) -> Metadata {
    let file = TransformFile::new(cm, comments);
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));
    Metadata::new(state)
  }

  fn create_file(code: &str) -> (Lrc<SourceMap>, Lrc<SourceFile>) {
    let cm: Lrc<SourceMap> = Default::default();
    let file = cm.new_source_file(Lrc::new(FileName::Custom("test.jsx".into())), code.into());
    (cm, file)
  }

  fn span_for(file: &SourceFile, start: usize, len: usize) -> Span {
    let base = file.start_pos.0;
    let lo = BytePos(base + start as u32);
    let hi = BytePos(base + (start + len) as u32);
    Span::new(lo, hi)
  }

  fn line_comment(file: &SourceFile, code: &str, marker: &str) -> Comment {
    let start = code.find(marker).expect("marker missing in code");
    let end = code[start..]
      .find('\n')
      .map(|idx| start + idx)
      .unwrap_or_else(|| code.len());
    let span = span_for(file, start, end - start);
    let text = code[start + 2..end].to_string();

    Comment {
      kind: CommentKind::Line,
      span,
      text: text.into(),
    }
  }

  fn block_comment(file: &SourceFile, code: &str, marker: &str) -> Comment {
    let start = code.find(marker).expect("marker missing in code");
    let span = span_for(file, start, marker.len());
    let text = marker
      .trim_start_matches("/*")
      .trim_end_matches("*/")
      .to_string();

    Comment {
      kind: CommentKind::Block,
      span,
      text: text.into(),
    }
  }

  #[test]
  fn returns_comments_for_same_line_and_previous_line() {
    let code = "\
            const value = css({ color: 'red' });\n\
            // before\n\
            identifier(); // current
        ";
    let (cm, file) = create_file(code);
    let mut comments = Vec::new();
    comments.push(line_comment(&file, code, "// before"));
    comments.push(line_comment(&file, code, "// current"));

    let identifier_start = code.find("identifier").expect("identifier missing");
    let span = span_for(&file, identifier_start, "identifier();".len());

    let metadata = build_metadata(cm.clone(), comments);
    let result = get_node_comments(span, &metadata);

    assert_eq!(result.current.len(), 1);
    assert!(result.current[0].text.contains("current"));
    assert_eq!(result.before.len(), 1);
    assert!(result.before[0].text.contains("before"));
  }

  #[test]
  fn ignores_block_and_multi_line_comments() {
    let code = "\
            const one = 1; /* block */\n\
            /* multi\n               line */\n\
            // before first line\n\
            identifier();
        ";
    let (cm, file) = create_file(code);
    let mut comments = Vec::new();
    comments.push(block_comment(&file, code, "/* block */"));
    comments.push(block_comment(
      &file,
      code,
      "/* multi\n               line */",
    ));
    comments.push(line_comment(&file, code, "// before first line"));

    let identifier_start = code.find("identifier").expect("identifier missing");
    let span = span_for(&file, identifier_start, "identifier();".len());

    let metadata = build_metadata(cm.clone(), comments);
    let result = get_node_comments(span, &metadata);

    assert!(result.current.is_empty());
    assert_eq!(result.before.len(), 1);
    assert!(result.before[0].text.contains("before"));
  }

  #[test]
  fn returns_empty_when_node_spans_multiple_lines() {
    let code = "\
            const value = css({\n              color: 'red'\n            }); // comment
        ";
    let (cm, file) = create_file(code);
    let mut comments = Vec::new();
    comments.push(line_comment(&file, code, "// comment"));

    let start = code.find("css({").expect("css call missing");
    let end = code.find("});").expect("closing not found") + 3;
    let span = span_for(&file, start, end - start);

    let metadata = build_metadata(cm.clone(), comments);
    let result = get_node_comments(span, &metadata);

    assert!(result.before.is_empty());
    assert!(result.current.is_empty());
  }
}
