use swc_core::common::Spanned;

use crate::types::TransformState;

/// Mirrors the behaviour of `@compiled/utils`'s `preserveLeadingComments` helper
/// by ensuring comments that originally preceded the first statement remain at the
/// top of the file after additional nodes are inserted ahead of the program body.
pub fn preserve_leading_comments<T>(items: &[T], state: &mut TransformState)
where
  T: Spanned,
{
  if state.file.comments.is_empty() {
    return;
  }

  let Some(first) = items.first() else {
    return;
  };

  let cutoff = first.span().lo();

  let (leading, trailing): (Vec<_>, Vec<_>) = state
    .file
    .comments
    .iter()
    .cloned()
    .partition(|comment| comment.span.hi <= cutoff);

  if leading.is_empty() {
    return;
  }

  state.file.comments = leading.into_iter().chain(trailing.into_iter()).collect();
}
