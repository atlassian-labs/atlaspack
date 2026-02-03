use parking_lot::Mutex;
use std::any;
use std::any::Any;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;

use napi::Env;
use napi::JsNumber;
use napi::NapiRaw;
use napi::bindgen_prelude::FromNapiValue;
use napi::bindgen_prelude::ToNapiValue;
use once_cell::sync::Lazy;

type Inner = Arc<dyn Any + Send + Sync>;

static COUNTER: AtomicI32 = AtomicI32::new(0);
static VALUES: Lazy<Mutex<HashMap<i32, Inner>>> = Lazy::new(Default::default);

/// Creates an external reference to a Rust value and
/// makes it transferable across Nodejs workers
///
/// This is to get around the limitations of what can be transferred
/// between workers in Nodejs
///
/// https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Transferable_objects
pub struct JsTransferable<T> {
  id: i32,
  _value: PhantomData<T>,
}

impl<T: Send + Sync + 'static> JsTransferable<T> {
  /// Put a Rust value into a Transferable container to allow
  /// sending values to Nodejs workers via postMessage or workerData
  pub fn new(value: T) -> Self {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);

    VALUES.lock().insert(id, Arc::new(value));
    Self {
      id,
      _value: Default::default(),
    }
  }

  /// Take the value out of Transferable, so it can no longer be accessed
  pub fn take(self) -> napi::Result<T> {
    let value = {
      let mut values = VALUES.lock();
      let Some(value) = values.remove(&self.id) else {
        return Err(napi::Error::from_reason(format!(
          "JsTransferableError::NotExists: id({})",
          self.id
        )));
      };
      value
    };

    let Ok(val) = value.downcast::<T>() else {
      return Err(napi::Error::from_reason(format!(
        "JsTransferableError::InvalidDowncast: id({}) type({})",
        self.id,
        any::type_name::<T>()
      )));
    };

    let Some(value) = Arc::into_inner(val) else {
      return Err(napi::Error::from_reason(format!(
        "JsTransferableError::StillBorrowed: id({})",
        self.id
      )));
    };

    Ok(value)
  }

  // Get a read-only copy of a Transferable value
  pub fn get(&self) -> napi::Result<JsTransferableRef<T>> {
    let values = VALUES.lock();

    let Some(value) = values.get(&self.id) else {
      return Err(napi::Error::from_reason(format!(
        "JsTransferableError::NotExists: id({})",
        self.id
      )));
    };
    let value = value.clone();
    let Ok(val) = value.downcast::<T>() else {
      return Err(napi::Error::from_reason(format!(
        "JsTransferableError::InvalidDowncast: id({}) type({})",
        self.id,
        any::type_name::<T>()
      )));
    };

    Ok(JsTransferableRef(val))
  }
}

/// Clear all entries from the transferable registry.
///
/// This should be called during worker pool shutdown to release any
/// Arc references that are stored in the global registry. Without this,
/// Rust-side values would persist in memory even after worker threads
/// are terminated.
pub fn clear_transferable_registry() {
  VALUES.lock().clear();
}

pub struct JsTransferableRef<T>(Arc<T>);

impl<T> Deref for JsTransferableRef<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.0.as_ref()
  }
}

/// Allows Transferable to be returned from a Napi functions
impl<T> ToNapiValue for JsTransferable<T> {
  unsafe fn to_napi_value(
    env: napi::sys::napi_env,
    val: Self,
  ) -> napi::Result<napi::sys::napi_value> {
    let env = unsafe { Env::from_raw(env) };
    let pointer = env.create_int32(val.id)?;
    Ok(unsafe { pointer.raw() })
  }
}

/// Allows Transferable to be accepted as an argument for a Napi function
impl<T> FromNapiValue for JsTransferable<T> {
  unsafe fn from_napi_value(
    env: napi::sys::napi_env,
    napi_val: napi::sys::napi_value,
  ) -> napi::Result<Self> {
    let pointer = unsafe { JsNumber::from_napi_value(env, napi_val) }?;
    let id = pointer.get_int32()?;
    Ok(Self {
      id,
      _value: Default::default(),
    })
  }
}
