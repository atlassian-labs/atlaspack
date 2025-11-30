pub mod parse;
pub mod stringify;
pub mod unit;
pub mod walk;

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
  Space {
    value: String,
  },
  String {
    value: String,
    quote: char,
    unclosed: bool,
  },
  Comment {
    value: String,
    unclosed: bool,
  },
  Word {
    value: String,
  },
  Function {
    value: String,
    nodes: Vec<Node>,
    before: String,
    after: String,
    unclosed: bool,
  },
  Div {
    value: String,
    before: String,
    after: String,
  },
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ParsedValue {
  pub nodes: Vec<Node>,
}

pub fn parse(input: &str) -> ParsedValue {
  parse::parse(input)
}
pub fn walk(nodes: &mut [Node], cb: &mut dyn FnMut(&mut Node) -> bool, bubble: bool) {
  walk::walk(nodes, cb, bubble)
}
pub fn stringify(nodes: &[Node]) -> String {
  stringify::stringify(nodes)
}
