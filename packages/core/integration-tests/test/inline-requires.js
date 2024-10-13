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

[false, true].forEach(value => {
  const implementation = value ? 'rust' : 'js';
  describe.v2(`inline requires - ${implementation}`, () => {
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
          featureFlags: {
            fastOptimizeInlineRequires: value,
          },
        },
      );
      const bundles = bundleGraph.getBundles();
      const mainBundle = bundles.find(b => b.name === 'index.js');
      const otherBundle = bundles.find(b => b.name.includes('other'));
      if (mainBundle == null) throw new Error('There was no JS bundle');
      if (otherBundle == null) throw new Error('There was no JS bundle');
      const bundleContents = overlayFS.readFileSync(
        mainBundle.filePath,
        'utf8',
      );
      const otherContentsRaw = overlayFS.readFileSync(
        otherBundle.filePath,
        'utf8',
      );

      const cleanRequires = (str: string) =>
        str.replace(/parcelRequire\([^)]*\)/g, 'parcelRequire(...)');

      const contents = cleanRequires(bundleContents);
      const otherContents = cleanRequires(otherContentsRaw);

      if (implementation === 'js') {
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
      } else {
        expect(otherContents).toContain(
          `console.log((0, parcelRequire(...).exportedFunction)())`,
        );
        expect(contents).toContain(
          `
    setTimeout(()=>{
        (0, parcelRequire(...).exportedFunction)();
    }, 5000);
      `.trim(),
        );
      }
    });
  });
});
