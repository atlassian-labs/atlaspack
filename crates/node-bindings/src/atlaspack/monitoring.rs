use napi_derive::napi;

#[napi]
pub fn initialize_monitoring() -> napi::Result<()> {
  atlaspack_monitoring::initialize_from_env()
    .map_err(|err| napi::Error::from_reason(err.to_string()))
}

#[napi]
pub fn close_monitoring() {
  atlaspack_monitoring::close_monitoring();
}

#[cfg(not(target_arch = "wasm32"))]
#[napi]
pub fn get_native_memory_stats() -> Option<atlaspack_memory_profiler::NativeMemoryStats> {
  atlaspack_memory_profiler::get_native_memory_stats()
}

#[cfg(not(target_arch = "wasm32"))]
#[napi]
pub fn reset_memory_tracking() {
  atlaspack_memory_profiler::reset_memory_tracking();
}

#[cfg(not(target_arch = "wasm32"))]
#[napi]
pub fn sample_native_memory() {
  atlaspack_memory_profiler::sample_native_memory();
}
