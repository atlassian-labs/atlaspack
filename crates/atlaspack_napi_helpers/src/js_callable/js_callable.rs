use std::sync::mpsc::channel;
use std::sync::Arc;
#[cfg(debug_assertions)]
use std::thread::ThreadId;

use napi::threadsafe_function::ErrorStrategy;
use napi::threadsafe_function::ThreadSafeCallContext;
use napi::threadsafe_function::ThreadsafeFunction;
use napi::threadsafe_function::ThreadsafeFunctionCallMode;
use napi::Env;
use napi::JsFunction;
use napi::JsObject;
use napi::JsUnknown;
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::map_params_serde;
use super::map_return_serde;
use super::JsValue;

pub type MapJsParams = Box<dyn FnOnce(&Env) -> napi::Result<Vec<JsUnknown>> + 'static>;
pub type MapJsReturn<Return> = Box<dyn Fn(&Env, JsUnknown) -> napi::Result<Return> + 'static>;

/// JsCallable provides a Send + Sync wrapper around callable JavaScript functions
///
/// Functions can be called from threads or the main thread, while parameters and return types can
/// be mapped by the caller.
pub struct JsCallable {
  #[cfg(debug_assertions)]
  initial_thread: ThreadId,
  fn_name: Arc<String>,
  threadsafe_function: ThreadsafeFunction<MapJsParams, ErrorStrategy::Fatal>,
}

impl std::fmt::Debug for JsCallable {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("JsCallable")
      .field("name", &self.fn_name)
      .finish()
  }
}

impl JsCallable {
  pub fn new(callback: JsFunction, fn_name: String) -> napi::Result<Self> {
    // Store the threadsafe function on the struct
    let tsfn: ThreadsafeFunction<MapJsParams, ErrorStrategy::Fatal> = callback
      .create_threadsafe_function(0, |ctx: ThreadSafeCallContext<MapJsParams>| {
        (ctx.value)(&ctx.env)
      })?;

    Ok(Self {
      #[cfg(debug_assertions)]
      initial_thread: std::thread::current().id(),
      fn_name: Arc::new(fn_name),
      threadsafe_function: tsfn,
    })
  }

  /// Construct a JsCallable from an object property
  pub fn new_method(method_name: &str, obj: &JsObject) -> napi::Result<Self> {
    Self::new(
      obj.get_named_property(method_name)?,
      method_name.to_string(),
    )
  }

  /// Construct a JsCallable from an object property, binding it to the source object
  pub fn new_method_bound(method_name: &str, obj: &JsObject) -> napi::Result<Self> {
    let jsfn: JsFunction = obj.get_named_property(method_name)?;
    let fn_obj = jsfn.coerce_to_object()?;
    let bind: JsFunction = fn_obj.get_named_property("bind")?;
    let jsfn: JsFunction = bind.call(Some(&fn_obj), &[obj])?.try_into()?;
    Self::new(jsfn, method_name.to_string())
  }

  pub fn into_unref(mut self, env: &Env) -> napi::Result<Self> {
    self.threadsafe_function.unref(env)?;
    Ok(self)
  }

  /// Call JavaScript function and handle the return value
  pub fn call<Return>(
    &self,
    map_params: impl FnOnce(&Env) -> napi::Result<Vec<JsUnknown>> + 'static,
    map_return: impl Fn(&Env, JsUnknown) -> napi::Result<Return> + 'static,
  ) -> napi::Result<Return>
  where
    Return: Send + 'static,
  {
    #[cfg(debug_assertions)]
    if self.initial_thread == std::thread::current().id() {
      return Err(napi::Error::from_reason(format!(
        "Cannot run threadsafe function {} on main thread",
        self.fn_name
      )));
    }

    let (tx, rx) = channel();

    self.threadsafe_function.call_with_return_value(
      Box::new(map_params),
      ThreadsafeFunctionCallMode::NonBlocking,
      {
        let fn_name = self.fn_name.clone();

        move |JsValue(value, env)| {
          if value.is_promise()? {
            let result: JsObject = value.try_into()?;
            let then_fn: JsFunction = result.get_named_property("then")?;

            let then_result_fn =
              env.create_function_from_closure("JsCallable::then_result_fn", {
                let tx = tx.clone();
                let fn_name = fn_name.clone();

                move |ctx| {
                  if tx.send(map_return(&env, ctx.get::<JsUnknown>(0)?)).is_err() {
                    return Err(napi::Error::from_reason(format!(
                      "JsCallable({}) SendError: Result.then()",
                      &fn_name
                    )));
                  }
                  ctx.env.get_undefined()
                }
              })?;

            let then_error_fn = env.create_function_from_closure("JsCallable::then_error_fn", {
              let tx = tx.clone();
              let fn_name = fn_name.clone();

              move |ctx| {
                let err = napi::Error::from(ctx.get::<JsUnknown>(0)?);
                if tx.send(Err(err)).is_err() {
                  return Err(napi::Error::from_reason(format!(
                    "JsCallable({}) SendError: Result.catch()",
                    &fn_name
                  )));
                };
                ctx.env.get_undefined()
              }
            })?;

            then_fn.call(Some(&result), &[then_result_fn, then_error_fn])?;
            return Ok(());
          }

          if value.is_error()? {
            if tx.send(Err(napi::Error::from(value))).is_err() {
              return Err(napi::Error::from_reason(format!(
                "JsCallable({}) SendError: Sync Result Thrown",
                &fn_name
              )));
            };
            return Ok(());
          }

          if tx.send(map_return(&env, value)).is_err() {
            return Err(napi::Error::from_reason(format!(
              "JsCallable({}) SendError: Sync Result",
              &fn_name
            )));
          };
          Ok(())
        }
      },
    );

    match rx.recv() {
      Ok(Ok(result)) => Ok(result),
      Ok(Err(err)) => Err(err),
      Err(err) => Err(napi::Error::from_reason(format!(
        "JsCallable({}) RecvError: {:?}",
        &self.fn_name, err
      ))),
    }
  }

  pub fn call_serde<Params, Return>(&self, params: Params) -> napi::Result<Return>
  where
    Params: Serialize + Send + Sync + 'static,
    Return: Send + DeserializeOwned + 'static,
  {
    self.call(map_params_serde(params), map_return_serde())
  }
}
