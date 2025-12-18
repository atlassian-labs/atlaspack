static ATLASPACK_RUST_VERSION: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

/// Returns the version of the @atlaspack/rust package as a u64 hash.
pub fn atlaspack_rust_version() -> u64 {
  *ATLASPACK_RUST_VERSION
    .get_or_init(|| env!("ATLASPACK_RUST_VERSION_HASH").parse::<u64>().unwrap())
}
