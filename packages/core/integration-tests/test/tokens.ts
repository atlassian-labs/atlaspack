import {bundle, fsFixture, overlayFS} from '@atlaspack/test-utils';
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
      firstBundle.includes('var(--ds-text, #172B4D)') ||
        firstBundle.includes('#172B4D'),
      `Expected transformed token value to be present, but bundle was ${firstBundle.substring(0, 200)}...`,
    );
  });
});
