use markup5ever_rcdom::Handle;

#[derive(PartialEq, Eq)]
pub enum DomTraversalOperation {
  Continue,
  Stop,
}

pub trait DomVisitor {
  fn visit_node(&mut self, node: Handle) -> DomTraversalOperation;
}

pub fn walk(node: Handle, visitor: &mut impl DomVisitor) {
  let mut queue = vec![node.clone()];
  while let Some(node) = queue.pop() {
    let operation = visitor.visit_node(node.clone());
    if operation == DomTraversalOperation::Stop {
      break;
    }

    let children = node.children.borrow();
    for child in children.iter() {
      queue.push(child.clone());
    }
  }
}
