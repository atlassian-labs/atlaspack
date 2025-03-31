extern crate napi_build;

fn main() {
  napi_build::setup();
  println!("cargo:rerun-if-changed=build.rs");
}
