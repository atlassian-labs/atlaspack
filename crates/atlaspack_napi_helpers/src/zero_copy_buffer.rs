use napi::*;

pub struct ZeroCopyBuffer(JsBuffer);

impl std::fmt::Debug for ZeroCopyBuffer {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "ZeroCopyBuffer")
  }
}

impl ZeroCopyBuffer {
  pub fn new(env: &Env, contents: Vec<u8>) -> napi::Result<Self> {
    let buffer = env.create_buffer_with_data(contents)?;
    let buffer = buffer.into_raw();
    Ok(Self(buffer))
  }

  pub fn to_vec(self) -> napi::Result<Vec<u8>> {
    Ok(self.0.into_value()?.to_vec())
  }

  pub fn into_unknown(self) -> JsUnknown {
    return self.0.into_unknown();
  }
}

impl NapiRaw for ZeroCopyBuffer {
  unsafe fn raw(&self) -> napi::sys::napi_value {
    self.0.raw()
  }
}

impl NapiValue for ZeroCopyBuffer {
  unsafe fn from_raw(env: napi::sys::napi_env, value: napi::sys::napi_value) -> napi::Result<Self> {
    let buffer = JsBuffer::from_raw(env, value)?;
    Ok(Self(buffer))
  }

  unsafe fn from_raw_unchecked(env: napi::sys::napi_env, value: napi::sys::napi_value) -> Self {
    let buffer = JsBuffer::from_raw_unchecked(env, value);
    Self(buffer)
  }
}
