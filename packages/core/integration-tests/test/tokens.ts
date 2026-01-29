import {it, bundle, overlayFS} from '@atlaspack/test-utils';
import assert from 'assert';
import path from 'path';

describe('tokens', () => {
  it('should no-op without config', async () => {
    // This test requires a real filesystem to work with V3, so we can't use fsFixture
    const b = await bundle(
      path.join(__dirname, './integration/tokens-no-config/index.js'),
      {
        inputFS: overlayFS,
        outputFS: overlayFS,
        mode: 'production',
      },
    );
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
    // This test requires a real filesystem to work with V3, so we can't use fsFixture
    const b = await bundle(
      path.join(__dirname, './integration/tokens/index.js'),
      {
        outputFS: overlayFS,
        mode: 'production',
      },
    );

    const firstBundle = await overlayFS.readFile(
      b.getBundles()[0].filePath,
      'utf8',
    );
    assert(
      firstBundle.includes('var(--ds-text, #172B4D)'),
      `Expected transformed token value to be present, but bundle was ${firstBundle.substring(0, 200)}...`,
    );
    assert(
      !firstBundle.includes('\\u2026'),
      'Expected … to not be munged, and \\u2026 not to be present',
    );
    assert(
      firstBundle.includes('…'),
      'Expected … to not be munged and present',
    );
  });

  it('should not transform tokens when the feature flag is disabled', async () => {
    // This test requires a real filesystem to work with V3, so we can't use fsFixture
    const b = await bundle(
      path.join(__dirname, './integration/tokens/index.js'),
      {
        outputFS: overlayFS,
        mode: 'production',
        featureFlags: {
          enableTokensTransformer: false,
        },
      },
    );

    const firstBundle = await overlayFS.readFile(
      b.getBundles()[0].filePath,
      'utf8',
    );

    require('fs').writeFileSync('/tmp/bundle.js', firstBundle);
    assert(
      !firstBundle.includes('var(--ds-text, #172B4D)'),
      `Expected token to not be transformed, but bundle contained <SNIP>...${firstBundle.substring(firstBundle.length - 400)}`,
    );
  });

  it.v2(
    'should fail when token() is called with invalid arguments',
    async function () {
      // This test verifies that the tokens transformer properly checks for errors
      // reported during transformation and fails the build when errors are found.
      //
      // Previously, there was a bug where errors reported via HANDLER.with(|h| h.span_err(...))
      // were collected but never checked after transformation, causing silent failures.
      // This test ensures that bug is fixed.
      //
      // We use a custom transformer (check-tokens-transformer.js) that runs after
      // the tokens transformer to simulate the Compiled CSS transformer behavior.
      // However, with the bug fixed, the tokens transformer should fail before
      // the check-tokens-transformer runs.
      try {
        await bundle(
          path.join(__dirname, './integration/tokens-silent-failure/index.js'),
          {
            outputFS: overlayFS,
            mode: 'development',
          },
        );

        // If we get here, the build succeeded when it should have failed
        assert.fail(
          `Build succeeded when it should have failed. ` +
            `The tokens transformer should have reported an error for token() with no arguments.`,
        );
      } catch (error: any) {
        // The build should fail because the tokens transformer detects the error
        // and properly reports it, causing the build to fail
        const errorMessage = error.message || error.toString();

        if (errorMessage.includes('token() requires at least one argument')) {
          // This is the expected behavior - the tokens transformer properly checks errors
          // and throws a ThrowableDiagnostic with the error message
          assert.ok(
            true,
            `Build failed as expected: tokens transformer detected and reported the error. ` +
              `Error: ${errorMessage}`,
          );
        } else if (errorMessage.includes('Found untransformed token() call')) {
          // This would indicate the bug still exists - the tokens transformer silently failed
          // and the check-tokens-transformer caught it
          assert.fail(
            `Build failed, but error was caught by check-tokens-transformer instead of tokens transformer. ` +
              `This suggests the silent failure bug may still exist. Error: ${errorMessage}`,
          );
        } else {
          // Some other error occurred - re-throw to see what it is
          throw error;
        }
      }
    },
  );
});
