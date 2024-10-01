import assert from 'assert';
import path from 'path';
import {bundle, describe, distDir, it, outputFS} from '@atlaspack/test-utils';

describe('tracing', function () {
  it('should produce a trace', async function () {
    await bundle(
      path.join(__dirname, '/integration/typescript-jsx/index.tsx'),
      {
        shouldTrace: true,
        targets: {
          default: {distDir},
        },
        additionalReporters: [
          {
            packageName: '@atlaspack/reporter-tracer',
            resolveFrom: __dirname,
          },
        ],
        outputFS,
      },
    );

    const files = outputFS.readdirSync(__dirname);
    // @ts-expect-error - TS7006 - Parameter 'file' implicitly has an 'any' type.
    const profileFile = files.find((file) => file.startsWith('parcel-trace'));
    assert(profileFile !== null);
    const content = await outputFS.readFile(
      path.join(__dirname, profileFile),
      'utf8',
    );
    const profileContent = JSON.parse(content + ']'); // Traces don't contain a closing ] as an optimisation for partial writes
    assert(profileContent.length > 0);
  });
});
