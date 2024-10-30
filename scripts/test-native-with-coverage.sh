#!/usr/bin/env bash

# Requires:
#
# - Linux
# - sudo apt install pkgconf libssl-dev
# - cargo install cargo-tarpaulin

cargo tarpaulin --skip-clean --workspace \
  --exclude lmdb-js-lite \
  --exclude atlaspack-node-bindings \
  --exclude atlaspack_napi_helpers \
  --exclude atlaspack_macros \
  --exclude atlaspack_plugin_rpc
