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
  it('should no-op without config', async () => {
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

        package.json:
          {
            "name": "tokens"
          }

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
    // Without config, the transformer should not change the code
    assert(
      firstBundle.includes('color.text'),
      `Expected the import to remain when no config is provided, but bundle was ${firstBundle.substring(0, 150)}...`,
    );
  });

  it('should transform tokens when valid fixture is provided', async () => {
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

        package.json:
          {
            "name": "tokens",
            "@atlaspack/transformer-tokens": {
              "tokenDataPath": "../../fixtures/tokens/token-data.json5"
            }
          }

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
      firstBundle.includes('var(--ds-text, #172B4D)') ||
        firstBundle.includes('#172B4D'),
      `Expected transformed token value to be present, but bundle was ${firstBundle.substring(0, 200)}...`,
    );
  });
});
