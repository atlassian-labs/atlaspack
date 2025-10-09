import {bundle, fsFixture, overlayFS} from '@atlaspack/test-utils';
import assert from 'assert';
import path from 'path';

/*

        package.json:
          {
            "name": "tokens",
            "@atlaspack/transformer-js": {
              "atlaskitTokens": {
                "tokenDataPath": "../../../../../../afm/tokens/platform/packages/design-system/tokens/src/artifacts/token-data.json"
              }
            }
          }
            */

describe('tokens', () => {
  it('should transform tokens', async () => {
    await fsFixture(overlayFS, __dirname)`
      tokens
        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.js": ["@atlaspack/transformer-tokens", "..."]
            }
          }

        index.js:
          import { token } from '@atlaskit/tokens';
          const v = token('color.text');
          console.log(v);

        yarn.lock: {}
        `;

    const b = await bundle(path.join(__dirname, 'tokens/index.js'), {
      inputFS: overlayFS,
      outputFS: overlayFS,
      mode: 'production',
    });
    const firstBundle = await overlayFS.readFile(
      b.getBundles()[0].filePath,
      'utf8',
    );
    assert(
      firstBundle.includes('var(--ds-text, #172B4D)'),
      `Expected var(--ds-text, #172B4D) to be in the bundle, but bundle was ${firstBundle.substring(0, 100)}...`,
    );
  });
});
