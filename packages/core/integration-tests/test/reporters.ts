import assert from 'assert';
import {execSync} from 'child_process';
import path from 'path';

import {bundler, describe, it} from '@atlaspack/test-utils';

describe('reporters', () => {
  let successfulEntry = path.join(
    __dirname,
    'integration',
    'reporters-success',
    'index.js',
  );

  let loadReporterFailureEntry = path.join(
    __dirname,
    'integration',
    'reporters-load-failure',
    'index.js',
  );

  let failingReporterEntry = path.join(
    __dirname,
    'integration',
    'reporters-failure',
    'index.js',
  );

  describe('running on the cli', () => {
    it('exit successfully when no errors are emitted', () => {
      assert.doesNotThrow(() =>
        execSync(
          `./node_modules/.bin/atlaspack build --no-cache ${successfulEntry}`,
          {
            stdio: 'ignore',
          },
        ),
      );
    });

    it('exit with an error code when a reporter fails to load', () => {
      assert.throws(() =>
        execSync(
          `./node_modules/.bin/atlaspack build --no-cache ${loadReporterFailureEntry}`,
          {
            stdio: 'ignore',
          },
        ),
      );
    });

    it('exit with an error code when a reporter emits an error', () => {
      assert.throws(() =>
        execSync(
          `./node_modules/.bin/atlaspack build --no-cache ${failingReporterEntry}`,
          {
            stdio: 'ignore',
          },
        ),
      );
    });
  });

  describe('running on the programmatic api', () => {
    it.v2('resolves when no errors are emitted', async () => {
      let buildEvent = await bundler(successfulEntry).run();

      assert.equal(buildEvent.type, 'buildSuccess');
    });

    it('rejects when a reporter fails to load', async () => {
      try {
        let buildEvent = await bundler(loadReporterFailureEntry).run();

        // @ts-expect-error - TS2345 - Argument of type 'BuildSuccessEvent' is not assignable to parameter of type 'string'.
        throw new Error(buildEvent);
      } catch (err: any) {
        assert.equal(err.name, 'Error');
        assert.deepEqual(
          // @ts-expect-error - TS7006 - Parameter 'd' implicitly has an 'any' type.
          err.diagnostics.map((d) => d.message),
          ['Cannot find Atlaspack plugin "./test-reporter"'],
        );
      }
    });

    it.v2('rejects when a reporter emits an error', async () => {
      try {
        let buildEvent = await bundler(failingReporterEntry).run();

        // @ts-expect-error - TS2345 - Argument of type 'BuildSuccessEvent' is not assignable to parameter of type 'string'.
        throw new Error(buildEvent);
      } catch (err: any) {
        assert.equal(err.name, 'BuildError');
        assert.deepEqual(
          // @ts-expect-error - TS7006 - Parameter 'd' implicitly has an 'any' type.
          err.diagnostics.map((d) => d.message),
          ['Failed to report buildSuccess'],
        );
      }
    });
  });
});
