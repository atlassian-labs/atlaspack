use std::sync::Arc;
use std::thread;

use atlaspack::AtlaspackError;
use napi::bindgen_prelude::FromNapiValue;
use napi::bindgen_prelude::ToNapiValue;
use napi::threadsafe_function::ErrorStrategy;
use napi::threadsafe_function::ThreadSafeCallContext;
use napi::threadsafe_function::ThreadsafeFunction;
use napi::threadsafe_function::ThreadsafeFunctionCallMode;
use napi::Env;
use napi::JsFunction;
use napi::JsObject;
use napi::JsUnknown;
use once_cell::sync::OnceCell;

pub trait NapiExt {
  fn tuple_ok(&self, value: impl ToNapiValue) -> napi::Result<JsObject>;
  fn tuple_err(&self, error: impl ToNapiValue) -> napi::Result<JsObject>;
  fn tuple_err_string(&self, error: &anyhow::Error) -> napi::Result<JsObject>;
  fn create_threaded_promise<ThreadFunc, NapiFunc, NapiRet>(
    &self,
    func: ThreadFunc,
  ) -> napi::Result<JsObject>
  where
    ThreadFunc: FnOnce() -> anyhow::Result<NapiFunc> + Send + 'static,
    NapiFunc: FnOnce(Env) -> napi::Result<NapiRet> + Send + 'static,
    NapiRet: ToNapiValue;
}

impl NapiExt for Env {
  /// This creates the following JavaScript tuple
  /// ```
  /// [JsAny, null]
  /// ```
  fn tuple_ok(&self, value: impl ToNapiValue) -> napi::Result<JsObject> {
    let mut obj = self.create_array(2)?;
    obj.set(0, value)?;
    obj.set(1, self.get_null())?;
    obj.coerce_to_object()
  }

  /// This creates the following JavaScript tuple
  /// ```
  /// [null, JsAny]
  /// ```
  fn tuple_err(&self, error: impl ToNapiValue) -> napi::Result<JsObject> {
    let mut obj = self.create_array(2)?;
    obj.set(0, self.get_null())?;
    obj.set(1, error)?;
    obj.coerce_to_object()
  }

  /// This casts an error to a string and returns it as a JavaScript tuple
  fn tuple_err_string(&self, error: &anyhow::Error) -> napi::Result<JsObject> {
    self.tuple_err(self.to_js_value(&AtlaspackError::from(error)))
  }

  /// Creates a system thread and returns a Promise back to JavaScript.
  /// Captures errors and returns them as a JavaScript tuple
  fn create_threaded_promise<ThreadFunc, NapiFunc, NapiRet>(
    &self,
    func: ThreadFunc,
  ) -> napi::Result<JsObject>
  where
    ThreadFunc: FnOnce() -> anyhow::Result<NapiFunc> + Send + 'static,
    NapiFunc: FnOnce(Env) -> napi::Result<NapiRet> + Send + 'static,
    NapiRet: ToNapiValue,
  {
    // Captures the executor function of Promise creation as a threadsafe function
    //   new Promise(resolve => {})
    //               -------  <- this bit
    let resolve_fn = Arc::new(OnceCell::new());

    // This is the callback supplied to `new Promise(executor)`
    let executor = self.create_function_from_closure("Promise::executor", {
      let resolve_fn = resolve_fn.clone();
      move |ctx| {
        let resolve: ThreadsafeFunction<MapJsParams, ErrorStrategy::Fatal> = ctx
          .get::<JsFunction>(0)?
          .create_threadsafe_function(0, |ctx: ThreadSafeCallContext<MapJsParams>| {
            Ok((ctx.value)(&ctx.env)?)
          })?;
        resolve_fn.set(resolve).ok();
        Ok(())
      }
    })?;

    // Construct a new Promise
    let promise_ctor: JsFunction = self.get_global()?.get_named_property("Promise")?;
    let promise = promise_ctor.new_instance(&[&executor])?;

    // Spawn a thread to execute the off-thread work
    // then calls the Promise.resolve function (threadsafe).
    // This casts the value to the result tuple before returning to JavaScript
    thread::spawn(move || {
      // Call the function on the new thread
      let result = func();

      // Process the return value on the JS thread
      resolve_fn.wait().call(
        Box::new(move |env| match result {
          Ok(value) => {
            // Execute the function passed in by the caller
            let data = match value(*env) {
              Ok(data) => data,
              Err(error) => {
                return Ok(vec![env.tuple_err(env.create_error(error))?.into_unknown()])
              }
            };

            // Safety: value is checked as being a ToNapiValue on the caller
            let js_value = unsafe {
              JsUnknown::from_napi_value(env.raw(), NapiRet::to_napi_value(env.raw(), data)?)?
            };

            Ok(vec![env.tuple_ok(js_value)?.into_unknown()])
          }
          // Capture errors and and return them as string values
          Err(err) => Ok(vec![env.tuple_err_string(&err)?.into_unknown()]),
        }),
        ThreadsafeFunctionCallMode::NonBlocking,
      );
    });

    Ok(promise)
  }
}

pub type MapJsParams = Box<dyn Send + FnOnce(&Env) -> anyhow::Result<Vec<JsUnknown>> + 'static>;
