use anyhow;

/// Convert anyhow error to napi error
pub fn anyhow_to_napi(error: anyhow::Error) -> napi::Error {
  napi::Error::from_reason(format!("[napi] {:?}", error))
}

pub fn anyhow_from_napi(value: napi::Error) -> anyhow::Error {
  anyhow::Error::msg(value.reason)
}
