use xxhash_rust::xxh3::Xxh3;
use xxhash_rust::xxh3::xxh3_64;

/// Atlaspack needs to use a hasher for generating certain identifiers used in caches.
///
/// The hashes don't need to be incredibly fast, but they should be stable across
/// runs, machines, platforms and versions.
///
/// These hashes will likely end-up being written to disk, either within output
/// JavaScript files or internal caches.
pub type IdentifierHasher = Xxh3;

/// Copy of one of the `node-bindings/src/hash.rs` functions.
pub fn hash_string(s: String) -> String {
  hash_bytes(s.as_bytes())
}

pub fn hash_bytes(s: &[u8]) -> String {
  let res = xxh3_64(s);
  format!("{:016x}", res)
}
