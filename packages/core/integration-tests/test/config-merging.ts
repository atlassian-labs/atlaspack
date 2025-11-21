import {
  bundle,
  describe,
  it,
  run,
  outputFS,
  fsFixture,
  inputFS,
} from '@atlaspack/test-utils';
import assert from 'assert';
import path from 'path';

describe('config merging', function () {
  it.v2('should merge incomplete config packages', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/config-merging/index.js'),
    );
    let content = (
      await outputFS.readFile(
        path.join(__dirname, '/integration/config-merging/dist/index.js'),
      )
    ).toString();
    assert(content.includes('runtime injected'));
    assert.equal((await run(b)).default, 'Hello world!');
  });

  it('should prioritize user config transformer patterns over base config patterns', async function () {
    let dir = path.join(__dirname, 'tmp');
    await inputFS.rimraf(dir);

    await fsFixture(inputFS, dir)`
      parcelrc-precedence
        .parcelrc:
          {
            "extends": ["@atlaspack/config-default"],
            "transformers": {
              "*.{svg,mp4}": ["./transformer.mjs"]
            }
          }

        yarn.lock:

        package.json:
          {
            "name": "parcelrc-precedence",
            "version": "1.0.0"
          }

        index.js:
          import svg from './icon.svg';

          export default svg;

        icon.svg:
          This wil be replaced

        transformer.mjs:
          import { Transformer } from '@atlaspack/plugin';

          export default new Transformer({
            transform({ asset }) {
              asset.type = 'js';
              asset.setCode('module.exports = "Transformed Asset";');
              return [asset];
            }
          });
    `;

    let b = await bundle(path.join(dir, 'parcelrc-precedence/index.js'));

    let result = await run(b);

    assert.equal(result.default, 'Transformed Asset');
  });
});
