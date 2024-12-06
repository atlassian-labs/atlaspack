use std::cell::RefCell;
use std::rc::Rc;

use napi::bindgen_prelude::ToNapiValue;
use napi::threadsafe_function::ErrorStrategy;
use napi::threadsafe_function::ThreadSafeCallContext;
use napi::threadsafe_function::ThreadsafeFunction;
use napi::threadsafe_function::ThreadsafeFunctionCallMode;
use napi::Env;
use napi::JsFunction;
use napi::JsObject;
use napi::JsUnknown;
use napi::NapiRaw;
use napi::NapiValue;
use serde::Serialize;

use crate::js_arc::JsArc;

/// JsResolvable is a Promise-like object that can be resolved
/// from threads it wasn't created on.
///
/// This is different to napi's `JsDeferred` type because the `.reject`
/// method is supplied with an `Env` and both the `.reject` and `.resolve`
/// methods can be called with different closure types which is useful
/// when the control flow can resolve in different places.
#[derive(Clone)]
pub struct JsResolvable {
  promise: JsArc<JsObject>,
  then_fn:
    ThreadsafeFunction<Box<dyn FnOnce(Env) -> napi::Result<JsUnknown>>, ErrorStrategy::Fatal>,
  catch_fn:
    ThreadsafeFunction<Box<dyn FnOnce(Env) -> napi::Result<JsUnknown>>, ErrorStrategy::Fatal>,
}

impl JsResolvable {
  pub fn new(env: &Env) -> napi::Result<Self> {
    // Store then/catch functions outside the Promise
    let then_fn = Rc::new(RefCell::new(None));
    let catch_fn = Rc::new(RefCell::new(None));

    // Construct Promise and capture the then/catch functions
    let executor = env.create_function_from_closure("napi::promise::executor", {
      let then_fn = then_fn.clone();
      let catch_fn = catch_fn.clone();

      move |ctx| {
        let resolve_func_js: JsFunction = ctx.get(0)?;
        let reject_func_js: JsFunction = ctx.get(1)?;

        let tsfn_then: ThreadsafeFunction<
          Box<dyn FnOnce(Env) -> napi::Result<JsUnknown>>,
          ErrorStrategy::Fatal,
        > = resolve_func_js.create_threadsafe_function(
          0,
          |ctx: ThreadSafeCallContext<Box<dyn FnOnce(Env) -> Result<JsUnknown, napi::Error>>>| {
            let func = ctx.value;
            let result = func(ctx.env.clone())?;
            Ok(vec![result])
          },
        )?;

        let tsfn_catch: ThreadsafeFunction<
          Box<dyn FnOnce(Env) -> napi::Result<JsUnknown>>,
          ErrorStrategy::Fatal,
        > = reject_func_js.create_threadsafe_function(
          0,
          |ctx: ThreadSafeCallContext<Box<dyn FnOnce(Env) -> Result<JsUnknown, napi::Error>>>| {
            let func = ctx.value;
            let result = func(ctx.env.clone())?;
            Ok(vec![result])
          },
        )?;

        then_fn.borrow_mut().replace(tsfn_then);
        catch_fn.borrow_mut().replace(tsfn_catch);

        Ok(())
      }
    })?;

    // Construct new Promise
    let promise_ctor: JsFunction = env.get_global()?.get_named_property("Promise")?;
    let promise = promise_ctor.new_instance(&[executor])?;
    let promise = JsArc::new(env, promise)?;

    // Extract then/catch functions
    let mut then_fn_cell = then_fn.borrow_mut();
    let mut catch_fn_cell = catch_fn.borrow_mut();

    Ok(Self {
      then_fn: then_fn_cell.take().unwrap(),
      catch_fn: catch_fn_cell.take().unwrap(),
      promise,
    })
  }

  pub fn resolve<F, N>(&self, mapper: F)
  where
    N: NapiRaw,
    F: FnOnce(Env) -> napi::Result<N> + 'static,
  {
    self.then_fn.call(
      JsResolvable::map_params(mapper),
      ThreadsafeFunctionCallMode::NonBlocking,
    );
  }

  pub fn resolve_value(&self, value: impl NapiRaw + 'static) {
    self.resolve(move |_env| Ok(value))
  }

  pub fn resolve_serde<Param: Serialize + 'static>(&self, input: Param) {
    self.resolve(move |env| env.to_js_value(&input))
  }

  pub fn reject<F, N>(&self, mapper: F)
  where
    N: NapiRaw,
    F: FnOnce(Env) -> napi::Result<N> + 'static,
  {
    self.catch_fn.call(
      JsResolvable::map_params(mapper),
      ThreadsafeFunctionCallMode::NonBlocking,
    );
  }

  pub fn reject_value(&self, value: impl NapiRaw + 'static) {
    self.reject(move |_env| Ok(value))
  }

  pub fn reject_serde<Param: Serialize + 'static>(&self, input: Param) {
    self.reject(move |env| env.to_js_value(&input))
  }

  fn map_params<F, N>(input: F) -> Box<dyn FnOnce(Env) -> napi::Result<JsUnknown>>
  where
    N: NapiRaw,
    F: FnOnce(Env) -> napi::Result<N> + 'static,
  {
    Box::new(move |env| -> napi::Result<JsUnknown> {
      let value = input(env)?;
      let value = unsafe { JsUnknown::from_raw(env.raw(), value.raw()) }?;
      Ok(value)
    })
  }
}

impl ToNapiValue for JsResolvable {
  unsafe fn to_napi_value(
    env: napi::sys::napi_env,
    val: Self,
  ) -> napi::Result<napi::sys::napi_value> {
    let env = unsafe { Env::from_raw(env) };
    Ok(val.promise.inner(&env)?.raw())
  }
}
