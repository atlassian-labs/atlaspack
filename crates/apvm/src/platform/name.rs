use base64::Engine;

pub fn encode<S: AsRef<str>>(input: S) -> anyhow::Result<String> {
  Ok(base64::prelude::BASE64_STANDARD.encode(input.as_ref()))
}

pub fn decode<S: AsRef<str>>(input: S) -> anyhow::Result<String> {
  Ok(String::from_utf8(
    base64::prelude::BASE64_STANDARD.decode(input.as_ref())?,
  )?)
}
