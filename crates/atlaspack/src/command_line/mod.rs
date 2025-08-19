use std::sync::atomic::{AtomicUsize, Ordering};

pub struct ProgressBar {
  message: String,
  current_position: AtomicUsize,
  total_length: AtomicUsize,
}

impl ProgressBar {
  pub fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
      current_position: AtomicUsize::new(0),
      total_length: AtomicUsize::new(0),
    }
  }

  pub fn inc(&self) {
    self.current_position.fetch_add(1, Ordering::Relaxed);
  }

  pub fn inc_length(&self) {
    self.total_length.fetch_add(1, Ordering::Relaxed);
  }

  pub fn set_length(&self, length: usize) {
    self.total_length.store(length, Ordering::Relaxed);
  }
}
