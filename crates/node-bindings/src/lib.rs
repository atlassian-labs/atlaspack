#![allow(dead_code)]

#[cfg(target_arch = "wasm32")]
use std::alloc::Layout;
#[cfg(target_arch = "wasm32")]
use std::alloc::alloc;

#[cfg(all(windows, not(miri)))]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// napi versions of `crate::core::requests`
#[cfg(not(target_arch = "wasm32"))]
mod fs_search;
mod hash;
#[cfg(not(target_arch = "wasm32"))]
mod image;
pub mod js_tracing;

#[cfg(not(target_arch = "wasm32"))]
mod atlaspack;
#[cfg(not(test))]
pub mod lmdb;
mod optimizers;
mod plugin_compiled_css_in_js;
mod resolver;
mod sourcemap;
mod string_ops;
mod transformer;
pub mod vcs;

mod plugin_tokens;

// Re-export memory profiling functions
pub use atlaspack_memory_profiler::{
  get_native_memory_stats, reset_memory_tracking, sample_native_memory,
};
pub use atlassian_swc_compiled_css::config::CompiledCssInJsTransformConfig;

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn napi_wasm_malloc(size: usize) -> *mut u8 {
  let align = std::mem::align_of::<usize>();
  if let Ok(layout) = Layout::from_size_align(size, align) {
    unsafe {
      if layout.size() > 0 {
        let ptr = alloc(layout);
        if !ptr.is_null() {
          return ptr;
        }
      } else {
        return align as *mut u8;
      }
    }
  }

  std::process::abort();
}

#[cfg(target_arch = "wasm32")]
mod wasm {
  use napi_derive::napi;

  #[link(wasm_import_module = "env")]
  extern "C" {
    fn log(ptr: *const u8, len: usize);
  }

  #[napi]
  pub fn init_panic_hook() {
    std::panic::set_hook(Box::new(|p| {
      let s = p.to_string();
      unsafe {
        log(s.as_ptr(), s.len());
      }
    }));
  }
}
