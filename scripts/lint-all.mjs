#!/usr/bin/env zx
/* eslint-disable no-console */
import { $ } from 'zx';

try {
  console.log("Linting js...")
  await $`eslint .`;
  console.log("Linting js successful!");

  console.log("Prettifying...")
  await $`prettier "./packages/*/*/{src,bin,test}/**/*.{js,json,md}" --list-different`;
  console.log("Prettifying successful!");

  console.log("Formatting rust...")
  await $`cargo fmt --all -- --check`;
  console.log("Rust formatting successful!");

  try {
    await $`cargo clippy --version`;
  } catch {
    console.log('cargo-clippy not found, installing...');
    await $`rustup component add clippy`;
  }
  console.log("Running clippy...")
  await $`cargo clippy -- -D warnings`;
  console.log("Clippy found no warnings!");
} catch (error) {

  console.error('An error occurred during the checks:', error);
  process.exit(1);
}