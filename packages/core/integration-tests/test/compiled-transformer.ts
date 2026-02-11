import assert from 'assert';
import path from 'path';
import {
  bundle,
  describe,
  fsFixture,
  it,
  outputFS,
  overlayFS,
} from '@atlaspack/test-utils';

describe('compiled transformer (babel-based)', function () {
  it('transforms compiled css-in-js code', async function () {
    const dir = path.join(__dirname, 'compiled-transformer-test');
    await overlayFS.mkdirp(dir);

    await fsFixture(overlayFS, dir)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.{js,mjs,jsm,jsx,es6,cjs,ts,tsx}": [
              "@atlaspack/transformer-compiled-external",
              "@atlaspack/transformer-compiled",
              "..."
            ]
          }
        }

      package.json:
        {
          "name": "compiled-test"
        }

      yarn.lock: {}

      index.jsx:
        import { css } from '@compiled/react';

        console.log('File START');

        const styles = css({ backgroundColor: 'green' });

        const App = () => (
          <>
            <div css={[{ fontSize: 50, color: 'red' }, styles]}>hello from atlaspack</div>
          </>
        );

        console.log('File END');

        export default App;
    `;

    const b = await bundle(path.join(dir, 'index.jsx'), {
      inputFS: overlayFS,
    });

    const bundles = b.getBundles();
    const jsBundle = bundles.find((bundle) => bundle.type === 'js');
    assert(jsBundle, 'Should have a JS bundle');

    const file = await outputFS.readFile(jsBundle.filePath, 'utf8');

    assert(
      !file.includes("Code was executed when it shouldn't have."),
      'Output should not contain "Code was executed when it shouldn\'t have."',
    );
  });
});
