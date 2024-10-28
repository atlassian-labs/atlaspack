//! This module contains a oneshot channel that can be cloned
use std::cell::Cell;
use std::rc::Rc;

use tokio::sync::oneshot::channel;
use tokio::sync::oneshot::Receiver;
use tokio::sync::oneshot::Sender as TokioSender;

pub struct SendError<T>(pub SendErrorKind, pub T);

#[derive(Debug)]
pub enum SendErrorKind {
  AlreadySent,
  Failure,
}

pub struct Sender<T> {
  inner: Rc<Cell<Option<TokioSender<T>>>>,
}

impl<T> Clone for Sender<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T> Sender<T> {
  pub fn send(&self, value: T) -> Result<(), SendError<T>> {
    let Some(tx) = self.inner.take().take() else {
      return Err(SendError(SendErrorKind::AlreadySent, value));
    };

    if let Err(value) = tx.send(value) {
      return Err(SendError(SendErrorKind::Failure, value));
    };

    Ok(())
  }
}

pub fn oneshot<T>() -> (Sender<T>, Receiver<T>) {
  let (tx, rx) = channel::<T>();
  let tx = Sender {
    inner: Rc::new(Cell::new(Some(tx))),
  };
  (tx, rx)
}
