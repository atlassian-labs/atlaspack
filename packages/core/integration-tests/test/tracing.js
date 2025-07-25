// @flow strict-local
import assert from 'assert';
import path from 'path';
import {bundle, describe, distDir, it, outputFS} from '@atlaspack/test-utils';
import {tracer} from '@atlaspack/profiler';

describe('tracing', function () {
  it('should produce a trace', async function () {
    await outputFS.mkdirp(path.join(__dirname, 'integration'));
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
    const profileFile = files.find((file) => file.startsWith('parcel-trace'));
    assert(profileFile !== null);
    const content = await outputFS.readFile(
      path.join(__dirname, profileFile),
      'utf8',
    );
    const profileContent = JSON.parse(content + ']'); // Traces don't contain a closing ] as an optimisation for partial writes
    assert(profileContent.length > 0);
    assert(
      !tracer.enabled,
      'Tracer should be disabled when Atlaspack is shut down',
    );
  });
});
