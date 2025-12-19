import {bundle, overlayFS} from '@atlaspack/test-utils';
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

  it('should silently fail when token() is called with invalid arguments', async () => {
    // This test demonstrates the silent failure bug: when the visitor reports errors
    // via HANDLER.with(|h| h.span_err(...)), those errors are collected but never
    // checked after transformation, causing the build to succeed even though tokens
    // weren't processed.

    // The bug: build succeeds even though token() has invalid syntax
    // The visitor reports an error but it's never checked, so the build succeeds
    // when it should fail. This is the silent failure we're testing for.
    //
    // We use a custom transformer (check-tokens-transformer.js) that runs after
    // the tokens transformer to simulate the Compiled CSS transformer behavior.
    // It throws if it finds any token() calls remaining, which should happen
    // when the tokens transformer silently fails.
    try {
      await bundle(
        path.join(__dirname, './integration/tokens-silent-failure/index.js'),
        {
          outputFS: overlayFS,
          mode: 'development',
        },
      );

      // If we get here, the build succeeded when it should have failed
      // This means the check-tokens-transformer didn't catch the untransformed token
      // which shouldn't happen if the bug exists (it should throw)
      assert.fail(
        `Build succeeded, but check-tokens-transformer should have thrown an error ` +
          `because token() calls remain untransformed. This suggests the bug may be fixed or the test setup is incorrect.`,
      );
    } catch (error: any) {
      // The build should fail because:
      // 1. The tokens transformer silently fails (doesn't transform token())
      // 2. The check-tokens-transformer finds the untransformed token() and throws
      // This demonstrates the silent failure bug - the tokens transformer should have
      // reported the error and failed the build, but instead it silently failed and
      // let the next transformer discover the problem.

      // Check if the error is from our check-tokens-transformer
      const errorMessage = error.message || error.toString();
      if (errorMessage.includes('Found untransformed token() call')) {
        // This is the expected behavior - the bug exists and our transformer caught it
        assert.ok(
          true,
          `Build failed as expected: check-tokens-transformer found untransformed token() calls. ` +
            `This demonstrates the silent failure bug - the tokens transformer should have failed ` +
            `with an error, but instead it silently failed and let the next transformer discover the problem.`,
        );
      } else if (
        errorMessage.includes('token() requires at least one argument')
      ) {
        // If we get this error, the bug is fixed - the tokens transformer is now properly
        // checking errors and failing the build
        throw new Error(
          `Build failed with tokens transformer error (bug is fixed!). ` +
            `The tokens transformer is now properly checking errors. Update this test. Error: ${errorMessage}`,
        );
      } else {
        // Some other error occurred
        throw error;
      }
    }
  });
});
