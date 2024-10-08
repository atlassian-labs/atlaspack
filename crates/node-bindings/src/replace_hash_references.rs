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
  // This performs no copying. If the buffer encoding is not UTF-8 we'll corrupt
  // the data, because we write UTF-8 strings into it without validating.
  let input_bytes = input.as_ref();
  let patterns: Vec<&String> = hash_ref_to_name_hash.keys().collect();
  let replacements: Vec<&String> = patterns
    .iter()
    .map(|pattern| hash_ref_to_name_hash.get(*pattern).unwrap())
    .collect();
  let ac = aho_corasick::AhoCorasick::new(patterns).map_err(|err| {
    napi::Error::new(
      Status::GenericFailure,
      format!("[napi] Failed to build aho-corasick replacer: {}", err),
    )
  })?;

  let output_string = ac.replace_all_bytes(input_bytes, &replacements);

  let buffer = Buffer::from(output_string);
  Ok(buffer)
}
