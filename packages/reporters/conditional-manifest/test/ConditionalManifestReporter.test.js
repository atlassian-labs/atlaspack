// @flow strict-local
import {
  manifestHashes,
  updateManifest,
} from '../src/ConditionalManifestReporter';
import sinon from 'sinon';
import {describe, it, beforeEach} from 'node:test';
import assert from 'assert';

let createMockOverlayFS = () => ({
  // Mock filesystem methods
  writeFile: sinon.stub(),
  readFile: sinon.stub(),
  exists: sinon.stub(),
  chdir: sinon.stub(),
  copyFile: sinon.stub(),
  createReadStream: sinon.stub(),
  createWriteStream: sinon.stub(),
  cwd: sinon.stub(),
  existsSync: sinon.stub(),
  findAncestorFile: sinon.stub(),
  findFirstFile: sinon.stub(),
  findNodeModule: sinon.stub(),
  getEventsSince: sinon.stub(),
  mkdirp: sinon.stub(),
  ncp: sinon.stub(),
  readFileSync: sinon.stub(),
  readdir: sinon.stub(),
  readdirSync: sinon.stub(),
  realpath: sinon.stub(),
  realpathSync: sinon.stub(),
  rimraf: sinon.stub(),
  stat: sinon.stub(),
  statSync: sinon.stub(),
  symlink: sinon.stub(),
  unlink: sinon.stub(),
  watch: sinon.stub(),
  writeSnapshot: sinon.stub(),
});

let createMockLogger = () => ({
  // Mock logger methods
  warn: sinon.stub(),
  info: sinon.stub(),
  verbose: sinon.stub(),
  error: sinon.stub(),
  log: sinon.stub(),
});

describe('ConditionalManifestReporter', function () {
  beforeEach(() => {
    manifestHashes.clear();
  });

  it('should write the manifest to file', async function () {
    let logger = createMockLogger();
    let overlayFS = createMockOverlayFS();

    const conditionalManifestFilename =
      '/project-root/dist/conditional-manifest.json';
    const conditionalManifest = JSON.stringify({test: 'manifest'});

    await updateManifest(
      overlayFS,
      logger,
      conditionalManifestFilename,
      conditionalManifest,
    );

    // Verify that writeFile was called with the correct arguments
    assert(
      overlayFS.writeFile.calledWith(
        conditionalManifestFilename,
        conditionalManifest,
        {mode: 0o666},
      ),
    );

    // Verify that logger.info was called with the correct message
    assert(
      logger.info.calledWith({
        message: `Wrote conditional manifest to ${conditionalManifestFilename}`,
      }),
    );
  });

  it('should not write the manifest if it has not changed', async function () {
    let logger = createMockLogger();
    let overlayFS = createMockOverlayFS();

    let conditionalManifestFilename =
      '/project-root/dist/conditional-manifest.json';
    let conditionalManifest = JSON.stringify({test: 'manifest'});

    await updateManifest(
      overlayFS,
      logger,
      conditionalManifestFilename,
      conditionalManifest,
    );

    await updateManifest(
      overlayFS,
      logger,
      conditionalManifestFilename,
      conditionalManifest,
    );

    // Verify that writeFile was called once
    assert(overlayFS.writeFile.calledOnce);

    // Verify that logger.info was called with the correct message
    assert(logger.info.calledOnce);
  });
});
