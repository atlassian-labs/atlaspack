/**
 * This is a workaround for https://github.com/rust-lang/rust/issues/91979
 * when running atlaspack with parcel bindings, it is possible that the parcel
 * dylib will be loaded from a node worker thread, which causes a crash on exit.
 *
 * This is a workaround to ensure that the parcel dylib is loaded in the main
 * thread, which fixes the crash.
 */
export function loadRustWorkerThreadDylibHack() {
  try {
    require('@parcel/rust'); // eslint-disable-line
  } catch (err: any) {
    /* ignore */
  }
}
