#!/usr/bin/env bash

set -e

if [ -f "rust-toolchain" ]; then
  # Use nvmrc
  rust_toolchain="--default-toolchain $(cat rust-toolchain)"
fi

install_dir="$HOME/.local/rust"

# Create Directories
rm -rf $install_dir
mkdir -p $install_dir/rust
mkdir -p $install_dir/rust/rustup
mkdir -p $install_dir/rust/cargo

# Setup Environment
export RUSTUP_HOME="$install_dir/rustup"
export CARGO_HOME="$install_dir/cargo"
export CARGO_BIN="$install_dir/cargo/bin"

echo "RUSTUP_HOME=$RUSTUP_HOME" >> $GITHUB_ENV
echo "CARGO_HOME=$CARGO_HOME" >> $GITHUB_ENV
echo "CARGO_BIN=$CARGO_BIN" >> $GITHUB_ENV

export PATH="$CARGO_BIN:$PATH"
echo "$CARGO_BIN" >> $GITHUB_PATH

# Install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path $rust_toolchain
