use std::{
  collections::VecDeque,
  sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::Sender,
    Arc,
  },
};

use petgraph::{
  algo::toposort, graph::NodeIndex, prelude::StableDiGraph, visit::EdgeRef, Direction,
};

trait ThreadPool {
  fn spawn(&self, task_id: TaskId, f: impl FnOnce() + Send + 'static);
}

struct RayonThreadPool;

impl ThreadPool for RayonThreadPool {
  fn spawn(&self, _task_id: TaskId, f: impl FnOnce() + Send + 'static) {
    rayon::spawn(f);
  }
}

pub struct Task<TaskData> {
  data: TaskData,
  pending: AtomicUsize,
}

/// Implements parallel graph processing similar to what is described in
/// this talk about Tracktion Graph DSP processing.
///
/// - https://www.youtube.com/watch?v=Mkz908eP_4g
///
/// ## Problem statement
///
/// We have a graph of tasks that needs to be processed, where certain tasks
/// may depend on other tasks being processed.
///
/// The dependencies are known ahead of time.
///
/// In a graph:
///
/// ```ignore
/// graph {
///   a -> b
///   a -> c
///
///   b -> d
///   c -> d
/// }
/// ```
///
/// We expect the following sequence of events:
///
/// 1. `a` is processed
/// 2. `b` and `c` are processed in parallel
/// 3. `d` is processed
///
/// We want this to work for arbitrary graphs of tasks.
pub struct ParallelGraphProcessor<TaskData> {
  graph: StableDiGraph<Arc<Task<TaskData>>, ()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(NodeIndex);

impl<TaskData: Send + Sync + 'static> ParallelGraphProcessor<TaskData> {
  pub fn new() -> Self {
    Self {
      graph: StableDiGraph::new(),
    }
  }

  pub fn add_task(&mut self, task: TaskData) -> TaskId {
    let node_index = self.graph.add_node(Arc::new(Task {
      data: task,
      pending: AtomicUsize::new(0),
    }));

    TaskId(node_index)
  }

  pub fn add_edge(&mut self, depends_on_task_id: TaskId, task_id: TaskId) {
    self.graph.add_edge(depends_on_task_id.0, task_id.0, ());

    self
      .graph
      .node_weight_mut(task_id.0)
      .unwrap()
      .pending
      .fetch_add(1, Ordering::Relaxed);
  }

  /// Run the task graph with the rayon thread-pool.
  pub fn run<R, F>(self, f: F)
  where
    R: Send + 'static,
    F: Fn(&TaskData) -> anyhow::Result<R> + Send + Sync + Copy + 'static,
  {
    self.run_with_thread_pool(RayonThreadPool, f)
  }

  fn run_with_thread_pool<R, F>(self, thread_pool: impl ThreadPool, f: F)
  where
    R: Send + 'static,
    F: Fn(&TaskData) -> anyhow::Result<R> + Send + Sync + Copy + 'static,
  {
    let topological_order = toposort(&self.graph, None).unwrap();

    let (tx, rx) = std::sync::mpsc::channel();

    let kick_off =
      |tx: Sender<(TaskId, anyhow::Result<R>)>, task_id: TaskId, task: Arc<Task<TaskData>>| {
        let tx = tx.clone();
        thread_pool.spawn(task_id, move || {
          let r = f(&task.data);
          tx.send((task_id, r)).unwrap();
        });
      };

    {
      for node_index in topological_order {
        let task = self.graph.node_weight(node_index).unwrap();

        if task.pending.load(Ordering::Relaxed) == 0 {
          kick_off(tx.clone(), TaskId(node_index), task.clone());
        }
      }
    }

    let mut remaining_tasks = self.graph.node_count();
    while remaining_tasks > 0 {
      let Ok((TaskId(node_index), _)) = rx.recv() else {
        break;
      };

      remaining_tasks -= 1;

      for edge in self.graph.edges_directed(node_index, Direction::Outgoing) {
        let pending_task = edge.target();

        let task = self.graph.node_weight(pending_task).unwrap();
        let current_pending = task.pending.fetch_sub(1, Ordering::Relaxed);
        if current_pending <= 1 {
          kick_off(tx.clone(), TaskId(pending_task), task.clone());
        }
      }
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  use rayon::ThreadPoolBuilder;
  use std::sync::{Mutex, OnceLock};
  use std::time::{Duration, Instant};

  #[test]
  fn test_order_and_parallelism() {
    static EVENTS: Mutex<Vec<Event>> = Mutex::new(vec![]);

    #[derive(Debug, Clone, PartialEq)]
    enum Event {
      Spawn(TaskId),
      Finish(TaskId),
    }

    struct MockThreadPool;
    impl ThreadPool for MockThreadPool {
      fn spawn(&self, task_id: TaskId, f: impl FnOnce() + Send + 'static) {
        EVENTS.lock().unwrap().push(Event::Spawn(task_id));

        std::thread::spawn(move || {
          std::thread::sleep(Duration::from_millis(100));
          // Run this before the callback, otherwise we're racing with the channel send
          // with the response.
          EVENTS.lock().unwrap().push(Event::Finish(task_id));

          f();
        });
      }
    }

    let mut processor = ParallelGraphProcessor::new();
    let task_a = processor.add_task("A");
    let task_b = processor.add_task("B");
    let task_c = processor.add_task("C");
    let task_d = processor.add_task("D");

    // graph {
    //   a -> b
    //   a -> c
    //   b -> d
    //   c -> d
    // }
    processor.add_edge(task_a, task_b);
    processor.add_edge(task_a, task_c);
    processor.add_edge(task_b, task_d);
    processor.add_edge(task_c, task_d);

    processor.run_with_thread_pool(MockThreadPool, |_| Ok(()));

    let log: Vec<Event> = EVENTS.lock().unwrap().clone();

    assert_eq!(log[0], Event::Spawn(task_a));
    assert_eq!(log[1], Event::Finish(task_a));

    let index = |event: &Event| log.iter().position(|e| e == event).unwrap();

    // check that B and C run concurrently
    // overall we should expect:
    // * A
    // * B, C
    // * D
    //
    // B, C might start/finish in arbitrary order.
    assert!(index(&Event::Spawn(task_b)) < index(&Event::Finish(task_c)));
    assert!(index(&Event::Spawn(task_c)) < index(&Event::Finish(task_b)));
    assert!(index(&Event::Finish(task_b)) < index(&Event::Spawn(task_d)));
    assert!(index(&Event::Finish(task_c)) < index(&Event::Spawn(task_d)));

    assert!(index(&Event::Spawn(task_d)) < index(&Event::Finish(task_d)));
  }
}
