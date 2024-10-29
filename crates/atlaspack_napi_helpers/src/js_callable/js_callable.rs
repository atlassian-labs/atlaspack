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
use tokio::sync::mpsc::unbounded_channel;

use super::map_params_serde;
use super::map_return_serde;
use super::JsValue;

pub type MapJsParams = Box<dyn Send + FnOnce(&Env) -> anyhow::Result<Vec<JsUnknown>> + 'static>;
pub type MapJsReturn<Return> =
  Box<dyn Send + Fn(&Env, JsUnknown) -> anyhow::Result<Return> + 'static>;

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
        Ok((ctx.value)(&ctx.env)?)
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
  pub async fn call<Return>(
    &self,
    map_params: impl Send + FnOnce(&Env) -> anyhow::Result<Vec<JsUnknown>> + 'static,
    map_return: impl Send + Fn(&Env, JsUnknown) -> anyhow::Result<Return> + 'static,
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

    let (tx, mut rx) = unbounded_channel::<napi::Result<Return>>();
    let send = {
      let name = self.fn_name.clone();
      move |error_message: &str, value: napi::Result<Return>| -> napi::Result<()> {
        if let Err(_) = tx.send(value) {
          return Err(napi::Error::from_reason(format!(
            "JsCallable({}): {}",
            &name, error_message
          )));
        }
        Ok(())
      }
    };

    /*
    The napi-rs thread safe function is already non blocking however moving the mapping
    functions into the closure triggers a rust bug caused by the compiler being unable to infer
    the lifetimes on auto impl traits.

    https://github.com/rust-lang/rust/issues/64552

    The simplest solution for now is to wrap the thread safe function call in a an async closure.
    In this case we are using `spawn_local` which creates a light weight tokio task on the
    current thread, having a negligible impact on performance.
    */
    tokio::task::spawn({
      let threadsafe_function = self.threadsafe_function.clone();
      async move {
        threadsafe_function.call_with_return_value(
          Box::new(map_params),
          ThreadsafeFunctionCallMode::NonBlocking,
          {
            move |JsValue(value, env)| {
              if value.is_promise()? {
                let result: JsObject = value.try_into()?;
                let then_fn: JsFunction = result.get_named_property("then")?;

                let then_result_fn =
                  env.create_function_from_closure("JsCallable::then_result_fn", {
                    let send = send.clone();
                    move |ctx| {
                      let return_value = ctx.get::<JsUnknown>(0)?;
                      let mapped = Ok(map_return(&ctx.env, return_value)?);
                      send("Result.then()", mapped)?;
                      ctx.env.get_undefined()
                    }
                  })?;

                let then_error_fn =
                  env.create_function_from_closure("JsCallable::then_error_fn", {
                    let send = send.clone();
                    move |ctx| {
                      let return_value = ctx.get::<JsUnknown>(0)?;
                      let err = napi::Error::from(return_value);
                      send("Result.catch()", Err(err))?;
                      ctx.env.get_undefined()
                    }
                  })?;

                then_fn.call(Some(&result), &[then_result_fn, then_error_fn])?;
                return Ok(());
              }

              match map_return(&env, value) {
                Ok(result) => send("Sync Result", Ok(result)),
                Err(err) => send("Sync Throw", Err(err.into())),
              }
            }
          },
        );
      }
    });

    match rx.recv().await {
      Some(Ok(result)) => Ok(result),
      Some(Err(err)) => Err(err),
      None => Err(napi::Error::from_reason(format!(
        "JsCallable({}) RecvError",
        &self.fn_name
      ))),
    }
  }

  /// Call JavaScript function and handle the return value
  pub fn call_blocking<Return>(
    &self,
    map_params: impl Send + FnOnce(&Env) -> anyhow::Result<Vec<JsUnknown>> + 'static,
    map_return: impl Send + Fn(&Env, JsUnknown) -> anyhow::Result<Return> + 'static,
  ) -> anyhow::Result<Return>
  where
    Return: Send + 'static,
  {
    #[cfg(debug_assertions)]
    if self.initial_thread == std::thread::current().id() {
      return Err(anyhow::anyhow!(
        "Cannot run threadsafe function {} on main thread",
        self.fn_name
      ));
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
                let return_value = ctx.get::<JsUnknown>(0)?;
                let err = napi::Error::from(return_value);
                if tx.send(Err(err.into())).is_err() {
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
            let err = napi::Error::from(value);
            if tx.send(Err(err.into())).is_err() {
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
      Ok(Err(err)) => Err(err.into()),
      Err(err) => Err(anyhow::anyhow!(
        "JsCallable({}) RecvError: {:?}",
        &self.fn_name,
        err
      )),
    }
  }

  pub fn call_serde_blocking<Params, Return>(&self, params: Params) -> anyhow::Result<Return>
  where
    Params: Serialize + Send + Sync + 'static,
    Return: Send + DeserializeOwned + 'static,
  {
    self.call_blocking(map_params_serde(params), map_return_serde())
  }

  pub async fn call_serde<Params, Return>(&self, params: Params) -> anyhow::Result<Return>
  where
    Params: Serialize + Send + Sync + 'static,
    Return: Send + DeserializeOwned + 'static,
  {
    Ok(
      self
        .call(map_params_serde(params), map_return_serde())
        .await?,
    )
  }
}
