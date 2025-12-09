use super::Node;

pub fn walk(nodes: &mut [Node], cb: &mut dyn FnMut(&mut Node) -> bool, bubble: bool) {
  let len = nodes.len();
  for i in 0..len {
    let node_ptr: *mut Node = &mut nodes[i] as *mut Node;
    let proceed = unsafe { cb(&mut *node_ptr) };
    if !bubble && !proceed {
      continue;
    }
    // Recurse into function nodes
    if let Node::Function {
      nodes: ref mut inner,
      ..
    } = nodes[i]
    {
      walk(inner, cb, bubble);
    }
    if bubble {
      unsafe {
        cb(&mut *node_ptr);
      }
    }
  }
}
