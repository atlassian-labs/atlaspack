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
  // This performs no copying. If the buffer encoding is not UTF-8 we'll corrupt
  // the data, because we write UTF-8 strings into it without validating.
  //
  // This is faster than using Regex, although it is not optimal because we
  // actually don't need to search for multiple strings, only for `HASH_REF_...`.
  //
  // Therefore this does a lot of unnecessary comparisons.
  //
  // It's possible to improve perf. by avoiding copying the buffer and replacing
  // in place as well as using a faster search strategy.
  //
  // However, we have found that the performance improvement is small. Also,
  // we have measured `daachorse` to be significantly slower than this
  // implementation.
  //
  // We have also measured using regex bytes and only searching for HASH_REF_...
  // to be slower than this implementation.
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
