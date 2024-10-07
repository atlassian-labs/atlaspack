use std::collections::HashMap;

use napi::bindgen_prelude::*;
use napi_derive::napi;

const HASH_REF_PREFIX: &str = "HASH_REF_";
const HASH_REF_HASH_LEN: usize = 16;

#[napi]
fn replace_hash_references(
  input: Buffer,
  hash_ref_to_name_hash: HashMap<String, String>,
) -> napi::Result<Buffer> {
  // We assume that the buffer is a UTF-8 string.
  //
  // This performs no copying but is still totally safe. It'll just throw if the
  // buffer is not a valid UTF-8 string.
  let input_bytes = input.as_ref();
  let input_string = std::str::from_utf8(input_bytes).map_err(|err| {
    napi::Error::new(
      Status::GenericFailure,
      format!(
        "[napi] Failed to parse input buffer as UTF-8 string: {}",
        err
      ),
    )
  })?;
  let regex = regex::Regex::new(r"HASH_REF_\w{16}").map_err(|err| {
    napi::Error::new(
      Status::GenericFailure,
      format!("[napi] Failed to compile regex: {}", err),
    )
  })?;

  let output_string = regex.replace_all(input_string, |captures: &regex::Captures| {
    let hash_ref = captures.get(0).unwrap().as_str();
    let name_hash = hash_ref_to_name_hash.get(hash_ref).unwrap();
    name_hash
  });

  let buffer = Buffer::from(output_string.as_bytes());

  Ok(buffer)
}
