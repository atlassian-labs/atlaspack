#!/usr/bin/env zx
import { $ } from 'zx';

try {
  // eslint-disable-next-line no-console
  console.log("Linting js...")
  await $`eslint .`;

  // eslint-disable-next-line no-console
  console.log("Prettifying...")
  await $`prettier "./packages/*/*/{src,bin,test}/**/*.{js,json,md}" --list-different`;

  // eslint-disable-next-line no-console
  console.log("Formatting rust...")
  await $`cargo fmt --all -- --check`;

  try {
    await $`cargo clippy --version`;
  } catch {
    // eslint-disable-next-line no-console
    console.log('cargo-clippy not found, installing...');
    await $`rustup component add clippy`;
  }
  // eslint-disable-next-line no-console
  console.log("Running clippy...")
  await $`cargo clippy`;
} catch (error) {

  // eslint-disable-next-line no-console
  console.error('An error occurred during the checks:', error);
  process.exit(1);
}