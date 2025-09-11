#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use petgraph::prelude::StableDiGraph;

  use super::*;

  #[test]
  fn test_shared_graph() {
    let mut graph: StableDiGraph<i32, ()> = StableDiGraph::new();

    let n1 = graph.add_node(1);
    let n2 = graph.add_node(2);

    let graph = Arc::new(graph);

    let t1 = std::thread::spawn({
      let graph = graph.clone();
      move || {
        let weight = graph.node_weight(n1);
        println!("weight: {:?}", weight);
      }
    });

    let t2 = std::thread::spawn({
      let graph = graph.clone();
      move || {
        let weight = graph.node_weight(n2);
        println!("weight: {:?}", weight);
      }
    });

    t1.join().unwrap();
    t2.join().unwrap();
  }
}
