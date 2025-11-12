import rootPath from '@af/root-path';
import { $, cd } from 'zx';

async function main() {
  const fixtureName = process.argv[2];
  if (!fixtureName) {
    console.error('Fixture name is required');
    process.exit(1);
  }

  await cd(`${rootPath.platformRoot()}/crates/compiled/crates/compiled_swc_plugin`);

  await $`./scripts/generate-babel-fixtures.js ${fixtureName}`;
  await $`cargo run --example generate_fixture ${fixtureName}`.nothrow();
}

main();