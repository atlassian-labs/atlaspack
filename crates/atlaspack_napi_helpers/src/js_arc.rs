use std::marker::PhantomData;
use std::ptr;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::ThreadId;

use napi::bindgen_prelude::FromNapiValue;
use napi::check_status;
use napi::sys;
use napi::threadsafe_function::ThreadsafeFunction;
use napi::threadsafe_function::ThreadsafeFunctionCallMode;
use napi::Env;
use napi::JsObject;
use napi::JsUnknown;
use napi::NapiRaw;
use napi::NapiValue;

/// JsArc is a container that allows JavaScript values to be Send
/// Inner values must be consumed on the thread that created them
pub struct JsArc<T> {
  pub raw_ref: sys::napi_ref,
  pub count: Arc<AtomicU32>,
  dropper: Arc<ThreadsafeFunction<JsArcAction>>,
  inner: PhantomData<T>,
  #[cfg(debug_assertions)]
  thread_id: ThreadId,
}

impl<T: NapiValue> JsArc<T> {
  pub fn new(env: &Env, target: T) -> napi::Result<Self> {
    // Container that will be referenced to hold inner value
    let mut container = env.create_array(1)?;
    container.set(0, target)?;

    // Create napi reference for the container
    let container_raw = unsafe { container.coerce_to_object()?.raw() };
    let count = Arc::new(AtomicU32::new(1));

    let mut raw_ref = ptr::null_mut();
    check_status!(unsafe {
      sys::napi_create_reference(env.raw(), container_raw, 1, &mut raw_ref)
    })?;

    // Create a ThreadSafe function to handle dropping & cloning
    let jsfn = env.create_function_from_closure::<Vec<JsUnknown>, _>("", |_ctx| Ok(vec![]))?;

    let mut tsfn = env.create_threadsafe_function::<JsArcAction, JsUnknown, _>(&jsfn, 0, {
      let count = count.clone();
      let raw_ref = raw_ref.clone() as usize;
      move |ctx| {
        let raw_ref = raw_ref.clone() as sys::napi_ref;
        let env_raw = ctx.env.raw();

        match ctx.value {
          JsArcAction::Clone => {
            // Increment the ref count
            let mut count: u32 = count.fetch_add(1, Ordering::Relaxed);
            check_status!(unsafe { sys::napi_reference_ref(env_raw, raw_ref, &mut count) })?;
          }
          JsArcAction::Drop => {
            // Decrement the ref count
            let mut count: u32 = count.fetch_sub(1, Ordering::Relaxed);
            let raw_ref = raw_ref;
            check_status!(unsafe { sys::napi_reference_unref(env_raw, raw_ref, &mut count) })?;
            if count == 0 {
              check_status!(unsafe { sys::napi_delete_reference(env_raw, raw_ref) })?;
            }
          }
        }

        Ok(vec![])
      }
    })?;

    tsfn.unref(env)?;

    Ok(Self {
      raw_ref,
      count,
      dropper: Arc::new(tsfn),
      inner: PhantomData::default(),
      #[cfg(debug_assertions)]
      thread_id: std::thread::current().id(),
    })
  }

  /// Get the JavaScript value contained within a JsArc.
  /// Can only be done on the thread that created the value
  pub fn inner(&self, env: &Env) -> napi::Result<T> {
    #[cfg(debug_assertions)]
    if self.thread_id != std::thread::current().id() {
      return Err(napi::Error::from_reason(format!(
        "JsArc inner value can only be obtained on the thread it was created on",
      )));
    }

    let mut result = ptr::null_mut();
    check_status!(
      unsafe { sys::napi_get_reference_value(env.raw(), self.raw_ref, &mut result) },
      "Failed to get reference value"
    )?;

    let container = unsafe { JsObject::from_napi_value(env.raw(), result) }?;
    container.get_element(0)
  }
}

// Tell the JavaScript thread to increase the ref count
impl<T> Clone for JsArc<T> {
  fn clone(&self) -> Self {
    self
      .dropper
      .call(Ok(JsArcAction::Clone), ThreadsafeFunctionCallMode::Blocking);

    Self {
      raw_ref: self.raw_ref.clone(),
      count: self.count.clone(),
      dropper: self.dropper.clone(),
      inner: self.inner.clone(),
      #[cfg(debug_assertions)]
      thread_id: self.thread_id.clone(),
    }
  }
}

impl<T> Drop for JsArc<T> {
  fn drop(&mut self) {
    self
      .dropper
      .call(Ok(JsArcAction::Drop), ThreadsafeFunctionCallMode::Blocking);
  }
}

unsafe impl<T> Send for JsArc<T> {}
unsafe impl<T> Sync for JsArc<T> {}

enum JsArcAction {
  Clone,
  Drop,
}
