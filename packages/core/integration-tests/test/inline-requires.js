// @flow strict-local

// $FlowFixMe
import expect from 'expect';
import path from 'path';
import {
  bundle,
  describe,
  fsFixture,
  it,
  overlayFS,
} from '@atlaspack/test-utils';
import {SourceMapConsumer} from 'source-map';

describe('inline requires', () => {
  let options = {
    defaultTargetOptions: {
      shouldScopeHoist: true,
      shouldOptimize: true,
    },
    mode: 'production',
  };

  it('inlines require statements', async () => {
    await fsFixture(overlayFS, __dirname)`
        inline-requires
          dependency/index.js:
            export function exportedFunction() {
              throw new Error("Shouldn't be called");
            }

          other.js:
            // this is here so that we don't scope hoist dependency/index.js
            import {exportedFunction} from './dependency';
            console.log(exportedFunction());

          index.js:
            import {exportedFunction} from './dependency';

            import('./other');

            setTimeout(() => {
              exportedFunction();
            }, 5000);

          dependency/package.json:
            {
              "sideEffects": false
            }

          .parcelrc:
            {
              "extends": "@atlaspack/config-default",
              "optimizers": {
                "*.js": ["@atlaspack/optimizer-inline-requires"]
              }
            }
    `;

    const bundleGraph = await bundle(
      path.join(__dirname, 'inline-requires/index.js'),
      {
        ...options,
        inputFS: overlayFS,
        config: path.join(__dirname, 'inline-requires/.parcelrc'),
      },
    );
    const bundles = bundleGraph.getBundles();
    const mainBundle = bundles.find((b) => b.name === 'index.js');
    const otherBundle = bundles.find((b) => b.name.includes('other'));
    if (mainBundle == null) throw new Error('There was no JS bundle');
    if (otherBundle == null) throw new Error('There was no JS bundle');
    const bundleContents = overlayFS.readFileSync(mainBundle.filePath, 'utf8');
    const otherContentsRaw = overlayFS.readFileSync(
      otherBundle.filePath,
      'utf8',
    );

    const cleanRequires = (str: string) =>
      str.replace(/parcelRequire\([^)]*\)/g, 'parcelRequire(...)');

    const contents = cleanRequires(bundleContents);
    const otherContents = cleanRequires(otherContentsRaw);

    expect(otherContents).toContain(
      `console.log((0, (0, parcelRequire(...)).exportedFunction)())`,
    );
    expect(contents).toContain(
      `
    setTimeout(()=>{
        (0, (0, parcelRequire(...)).exportedFunction)();
    }, 5000);
      `.trim(),
    );
  });

  it('keeps source-maps working', async () => {
    await fsFixture(overlayFS, __dirname)`
        inline-requires
          dependency/index.js:
            export function exportedFunction() {
              throw new Error("Shouldn't be called");
            }

          other.js:
            // this is here so that we don't scope hoist dependency/index.js
            import {exportedFunction} from './dependency';
            console.log(exportedFunction());

          index.js:
            import {exportedFunction} from './dependency';

            import('./other');

            setTimeout(() => {
              exportedFunction();
              console.log('line 7');
            }, 5000);

          dependency/package.json:
            {
              "sideEffects": false
            }

          .parcelrc:
            {
              "extends": "@atlaspack/config-default",
              "optimizers": {
                "*.js": ["@atlaspack/optimizer-inline-requires"]
              }
            }
    `;

    const bundleGraph = await bundle(
      path.join(__dirname, 'inline-requires/index.js'),
      {
        ...options,
        inputFS: overlayFS,
        config: path.join(__dirname, 'inline-requires/.parcelrc'),
        defaultTargetOptions: {
          sourceMaps: true,
        },
      },
    );
    const bundles = bundleGraph.getBundles();
    const mainBundle = bundles.find((b) => b.name === 'index.js');
    const otherBundle = bundles.find((b) => b.name.includes('other'));
    if (mainBundle == null) throw new Error('There was no JS bundle');
    if (otherBundle == null) throw new Error('There was no JS bundle');
    const bundleContents = overlayFS.readFileSync(mainBundle.filePath, 'utf8');
    const sourceMapContents = overlayFS.readFileSync(
      mainBundle.filePath + '.map',
      'utf8',
    );

    const sourceMapConsumer = await new SourceMapConsumer(
      JSON.parse(sourceMapContents),
    );

    const line =
      bundleContents.split('\n').findIndex((line) => line.includes('line 7')) +
      1;
    expect(line).not.toEqual(-1);
    const originalPosition = sourceMapConsumer.originalPositionFor({
      line,
      column: 'console.log'.length + 3,
    });
    expect(originalPosition.line).toBe(7);
    expect(originalPosition.source).toBe('inline-requires/index.js');
  });
});
