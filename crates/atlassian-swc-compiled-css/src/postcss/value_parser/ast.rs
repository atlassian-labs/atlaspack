use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum NodeData {
  Word(WordNode),
  Space(SpaceNode),
  String(StringNode),
  Comment(CommentNode),
  Div(DivNode),
  Function(FunctionNode),
  UnicodeRange(UnicodeRangeNode),
}

pub type Node = Rc<RefCell<NodeData>>;

#[derive(Clone, Debug, PartialEq)]
pub struct WordNode {
  pub value: String,
  pub source_index: usize,
  pub source_end_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpaceNode {
  pub value: String,
  pub source_index: usize,
  pub source_end_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StringNode {
  pub value: String,
  pub quote: Option<char>,
  pub unclosed: bool,
  pub source_index: usize,
  pub source_end_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommentNode {
  pub value: String,
  pub unclosed: bool,
  pub source_index: usize,
  pub source_end_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DivNode {
  pub value: String,
  pub before: String,
  pub after: String,
  pub source_index: usize,
  pub source_end_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionNode {
  pub value: String,
  pub before: String,
  pub after: String,
  pub nodes: Vec<Node>,
  pub unclosed: bool,
  pub source_index: usize,
  pub source_end_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnicodeRangeNode {
  pub value: String,
  pub source_index: usize,
  pub source_end_index: usize,
}

pub fn new_node(data: NodeData) -> Node {
  Rc::new(RefCell::new(data))
}

pub fn clone_nodes(nodes: &[Node]) -> Vec<Node> {
  nodes.iter().cloned().collect()
}
