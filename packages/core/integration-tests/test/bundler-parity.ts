import path from 'path';
import assert from 'assert';
import {
  bundle,
  overlayFS,
  fsFixture,
  generateSyntheticApp,
  describe,
  it,
} from '@atlaspack/test-utils';

type BundleStructure = Array<{type: string; assets: string[]}>;

async function compareBundlers(fixtureName: string, entryFile: string) {
  const entryPath = path.join(__dirname, fixtureName, entryFile);
  const commonOpts = {
    mode: 'development' as const,
    defaultTargetOptions: {
      shouldScopeHoist: false,
    },
    inputFS: overlayFS,
  };

  let jsBundleGraph = await bundle(entryPath, {
    ...commonOpts,
    featureFlags: {nativeBundling: false},
  });

  let rustBundleGraph = await bundle(entryPath, {
    ...commonOpts,
    featureFlags: {nativeBundling: true},
  });

  function extractBundles(bg: any): BundleStructure {
    let bundles: BundleStructure = [];
    bg.traverseBundles((b: any) => {
      let assets: string[] = [];
      b.traverseAssets((a: any) => {
        // Keep comparison stable across systems by comparing basenames only.
        let name = path.basename(a.filePath);

        // Skip runtime/helper assets that can differ.
        if (/@swc[/\\]helpers/.test(a.filePath)) return;
        if (/runtime-[a-z0-9]{16}\.js/.test(a.filePath)) return;

        // Skip JS runtime loader helpers that can be included differently depending on entry shape.
        if (
          name === 'bundle-url.js' ||
          name === 'cacheLoader.js' ||
          name === 'js-loader.js' ||
          name === 'esmodule-helpers.js'
        ) {
          return;
        }

        assets.push(name);
      });
      bundles.push({type: b.type, assets: assets.sort()});
    });

    // Sort bundles deterministically so deepEqual compares structure, not traversal order.
    bundles.sort((a, b) => {
      const aKey = a.type + ':' + a.assets.join(',');
      const bKey = b.type + ':' + b.assets.join(',');
      return aKey < bKey ? -1 : aKey > bKey ? 1 : 0;
    });

    return bundles;
  }

  let jsBundles = extractBundles(jsBundleGraph);
  let rustBundles = extractBundles(rustBundleGraph);

  // Compute diff between bundle sets for readable error messages.
  const toKey = (b: {assets: string[]}) => b.assets.join(',');
  const jsKeys = new Set(jsBundles.map(toKey));
  const rustKeys = new Set(rustBundles.map(toKey));
  const onlyInJs = jsBundles.filter((b) => !rustKeys.has(toKey(b)));
  const onlyInRust = rustBundles.filter((b) => !jsKeys.has(toKey(b)));

  const diffMsg =
    onlyInJs.length || onlyInRust.length
      ? `\n\nOnly in JS (${onlyInJs.length}):\n${onlyInJs.map((b) => `  [${b.assets.join(', ')}]`).join('\n')}` +
        `\n\nOnly in Rust (${onlyInRust.length}):\n${onlyInRust.map((b) => `  [${b.assets.join(', ')}]`).join('\n')}`
      : '';

  assert.equal(
    jsBundles.length,
    rustBundles.length,
    `Bundle count mismatch for ${fixtureName}: JS=${jsBundles.length}, Rust=${rustBundles.length}${diffMsg}`,
  );

  assert.deepEqual(
    rustBundles,
    jsBundles,
    `Bundle structure mismatch for ${fixtureName}.${diffMsg}`,
  );
}

describe.v3('bundler parity (js vs native)', function () {
  it('Simple shared module: two async entry points that both import the same module', async function () {
    const fixtureName = 'bundler-parity-simple-shared';
    const entryFile = `${fixtureName}-index.js`;
    const fooFile = `${fixtureName}-foo.js`;
    const barFile = `${fixtureName}-bar.js`;
    const sharedFile = `${fixtureName}-shared.js`;

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./${fooFile}');
          import('./${barFile}');
          export default 1;
        ${fooFile}:
          import shared from './${sharedFile}';
          export default shared + 'foo';
        ${barFile}:
          import shared from './${sharedFile}';
          export default shared + 'bar';
        ${sharedFile}:
          export default 'shared';
        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

    await compareBundlers(fixtureName, entryFile);
  });

  it('Diamond dependency: Entry → async A, async B → both import shared C and D', async function () {
    const fixtureName = 'bundler-parity-diamond';
    const entryFile = `${fixtureName}-index.js`;
    const aFile = `${fixtureName}-a.js`;
    const bFile = `${fixtureName}-b.js`;
    const cFile = `${fixtureName}-c.js`;
    const dFile = `${fixtureName}-d.js`;

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./${aFile}');
          import('./${bFile}');
          export default 1;
        ${aFile}:
          import c from './${cFile}';
          import d from './${dFile}';
          export default c + d + 'a';
        ${bFile}:
          import c from './${cFile}';
          import d from './${dFile}';
          export default c + d + 'b';
        ${cFile}:
          export default 'c';
        ${dFile}:
          export default 'd';
        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

    await compareBundlers(fixtureName, entryFile);
  });

  it('Transitive shared: Entry → async A → sync B and Entry → async C → sync B', async function () {
    const fixtureName = 'bundler-parity-transitive-shared';
    const entryFile = `${fixtureName}-index.js`;
    const aFile = `${fixtureName}-a.js`;
    const cFile = `${fixtureName}-c.js`;
    const bFile = `${fixtureName}-b.js`;

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./${aFile}');
          import('./${cFile}');
          export default 1;
        ${aFile}:
          import b from './${bFile}';
          export default b + 'a';
        ${cFile}:
          import b from './${bFile}';
          export default b + 'c';
        ${bFile}:
          export default 'b';
        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

    await compareBundlers(fixtureName, entryFile);
  });

  it('Mixed types: JS entry imports CSS', async function () {
    const fixtureName = 'bundler-parity-mixed-types';
    const entryFile = `${fixtureName}-index.js`;
    const cssFile = `${fixtureName}-style.css`;

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import './${cssFile}';
          export default 1;
        ${cssFile}:
          .${fixtureName} { color: red; }
        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

    await compareBundlers(fixtureName, entryFile);
  });

  it('Deep async chain: Entry → async A → async B → async C, with a module shared by A and C', async function () {
    const fixtureName = 'bundler-parity-deep-async-chain';
    const entryFile = `${fixtureName}-index.js`;
    const aFile = `${fixtureName}-a.js`;
    const bFile = `${fixtureName}-b.js`;
    const cFile = `${fixtureName}-c.js`;
    const sharedFile = `${fixtureName}-shared.js`;

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./${aFile}');
          export default 1;
        ${aFile}:
          import shared from './${sharedFile}';
          import('./${bFile}');
          export default shared + 'a';
        ${bFile}:
          import('./${cFile}');
          export default 'b';
        ${cFile}:
          import shared from './${sharedFile}';
          export default shared + 'c';
        ${sharedFile}:
          export default 'shared';
        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

    await compareBundlers(fixtureName, entryFile);
  });

  it('Many shared roots: 4+ async imports with overlapping shared subsets', async function () {
    const fixtureName = 'bundler-parity-many-shared-roots';
    const entryFile = `${fixtureName}-index.js`;

    const aFile = `${fixtureName}-a.js`;
    const bFile = `${fixtureName}-b.js`;
    const cFile = `${fixtureName}-c.js`;
    const dFile = `${fixtureName}-d.js`;

    const s1File = `${fixtureName}-s1.js`;
    const s2File = `${fixtureName}-s2.js`;
    const s3File = `${fixtureName}-s3.js`;
    const s4File = `${fixtureName}-s4.js`;
    const leaf1File = `${fixtureName}-leaf1.js`;
    const leaf2File = `${fixtureName}-leaf2.js`;
    const leaf3File = `${fixtureName}-leaf3.js`;
    const leaf4File = `${fixtureName}-leaf4.js`;

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./${aFile}');
          import('./${bFile}');
          import('./${cFile}');
          import('./${dFile}');
          export default 1;

        # Overlap matrix:
        # A: s1, s2
        # B: s2, s3
        # C: s1, s3, s4
        # D: s3, s4

        ${aFile}:
          import s1 from './${s1File}';
          import s2 from './${s2File}';
          import leaf1 from './${leaf1File}';
          export default s1 + s2 + leaf1;
        ${bFile}:
          import s2 from './${s2File}';
          import s3 from './${s3File}';
          import leaf2 from './${leaf2File}';
          export default s2 + s3 + leaf2;
        ${cFile}:
          import s1 from './${s1File}';
          import s3 from './${s3File}';
          import s4 from './${s4File}';
          import leaf3 from './${leaf3File}';
          export default s1 + s3 + s4 + leaf3;
        ${dFile}:
          import s3 from './${s3File}';
          import s4 from './${s4File}';
          import leaf4 from './${leaf4File}';
          export default s3 + s4 + leaf4;

        ${s1File}:
          export default 's1';
        ${s2File}:
          export default 's2';
        ${s3File}:
          export default 's3';
        ${s4File}:
          export default 's4';

        ${leaf1File}:
          export default 'leaf1';
        ${leaf2File}:
          export default 'leaf2';
        ${leaf3File}:
          export default 'leaf3';
        ${leaf4File}:
          export default 'leaf4';

        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

    await compareBundlers(fixtureName, entryFile);
  });

  it('Complex async graph with shared bundles should not crash during naming', async function () {
    const fixtureName = 'bundler-parity-complex-async-shared';
    const entryFile = 'index.js';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./page-a.js');
          import('./page-b.js');
          import('./page-c.js');

        page-a.js:
          import { helper } from './utils/helper.js';
          import { format } from './utils/format.js';
          export default helper() + format();

        page-b.js:
          import { helper } from './utils/helper.js';
          import { validate } from './utils/validate.js';
          export default helper() + validate();

        page-c.js:
          import { format } from './utils/format.js';
          import { validate } from './utils/validate.js';
          export default format() + validate();

        utils/helper.js:
          import { common } from './common.js';
          export function helper() { return common() + 'helper'; }

        utils/format.js:
          import { common } from './common.js';
          export function format() { return common() + 'format'; }

        utils/validate.js:
          import { common } from './common.js';
          export function validate() { return common() + 'validate'; }

        utils/common.js:
          export function common() { return 'common'; }

        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

    await compareBundlers(fixtureName, entryFile);
  });

  it('Async graph with type-change siblings should not crash during naming', async function () {
    const fixtureName = 'bundler-parity-async-type-change-siblings';
    const entryFile = 'index.js';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./page-a.js');
          import('./page-b.js');
          import('./page-c.js');

        page-a.js:
          import './styles/page-a.css';
          import './styles/shared.css';
          import { helper } from './utils/helper.js';
          import { format } from './utils/format.js';
          export default helper() + format();

        page-b.js:
          import './styles/page-b.css';
          import { helper } from './utils/helper.js';
          import { validate } from './utils/validate.js';
          export default helper() + validate();

        page-c.js:
          import { format } from './utils/format.js';
          import { validate } from './utils/validate.js';
          export default format() + validate();

        utils/helper.js:
          import { common } from './common.js';
          export function helper() { return common() + 'helper'; }

        utils/format.js:
          import { common } from './common.js';
          export function format() { return common() + 'format'; }

        utils/validate.js:
          import { common } from './common.js';
          export function validate() { return common() + 'validate'; }

        utils/common.js:
          export function common() { return 'common'; }

        styles/page-a.css:
          @import './shared.css';
          .page-a { color: red; }

        styles/page-b.css:
          @import './shared.css';
          .page-b { color: blue; }

        styles/shared.css:
          .shared { margin: 0; }

        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

    await compareBundlers(fixtureName, entryFile);
  });

  it('Generated app: ~150 assets with realistic structure', async function () {
    this.timeout(60000);
    const {fixtureName, entryFile} = await generateSyntheticApp(
      overlayFS,
      __dirname,
      150,
      42,
    );
    await compareBundlers(fixtureName, entryFile);
  });

  it('Generated app: ~1000 assets with realistic structure', async function () {
    this.timeout(300000);
    const {fixtureName, entryFile} = await generateSyntheticApp(
      overlayFS,
      __dirname,
      1000,
      123,
    );
    await compareBundlers(fixtureName, entryFile);
  });

  it('async import inside shared bundle should not crash during naming', async function () {
    // Repro for: bundle exists in a bundle group but is unreachable via traverseBundles,
    // so it never gets named by the Namer and crashes later in JSRuntime.getLoaderForBundle.
    const fixtureName = 'bundler-parity-async-in-shared';
    const entryFile = `${fixtureName}-index.js`;
    const asyncAFile = `${fixtureName}-async-a.js`;
    const asyncBFile = `${fixtureName}-async-b.js`;
    const sharedFile = `${fixtureName}-shared.js`;
    const localeFile = `${fixtureName}-locale.js`;

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          // Two async boundaries that both import the same shared module.
          import('./${asyncAFile}');
          import('./${asyncBFile}');

        ${sharedFile}:
          export const shared = 1;

          // Async import from within a module that should end up in a shared bundle.
          import('./${localeFile}');

        ${asyncAFile}:
          import {shared} from './${sharedFile}';

          export default shared;

        ${asyncBFile}:
          import {shared} from './${sharedFile}';

          export default shared;

        ${localeFile}:
          export const locale = 'hello';

        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

    await compareBundlers(fixtureName, entryFile);
  });
});
