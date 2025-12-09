use swc_core::common::Span;

/// Mirrors the Babel traverser result tuple by pairing a discovered node with
/// its span so that future passes can recover contextual metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraverserResult<T> {
  pub node: T,
  pub span: Span,
}
