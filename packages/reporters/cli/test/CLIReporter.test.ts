import assert from 'assert';
// @ts-expect-error - TS7016 - Could not find a declaration file for module 'sinon'. '/home/ubuntu/parcel/node_modules/sinon/lib/sinon.js' implicitly has an 'any' type.
import sinon from 'sinon';
import {PassThrough} from 'stream';
import {_report} from '../src/CLIReporter';
import * as render from '../src/render';
import {_setStdio} from '../src/render';
import {inputFS, outputFS} from '@atlaspack/test-utils';
import {NodePackageManager} from '@atlaspack/package-manager';
import stripAnsi from 'strip-ansi';
import * as bundleReport from '../src/bundleReport';
import {DEFAULT_FEATURE_FLAGS} from '@atlaspack/feature-flags';

const EMPTY_OPTIONS = {
  cacheDir: '.parcel-cache',
  parcelVersion: '',
  entries: [],
  logLevel: 'info',
  targets: [],
  projectRoot: '',
  distDir: 'dist',
  lockFile: undefined,
  shouldAutoInstall: false,
  shouldBuildLazily: false,
  hmrOptions: undefined,
  serveOptions: false,
  mode: 'development',
  shouldScopeHoist: false,
  shouldOptimize: false,
  env: {},
  shouldDisableCache: false,
  sourceMaps: false,
  inputFS,
  outputFS,
  instanceId: 'test',
  packageManager: new NodePackageManager(inputFS, '/'),
  detailedReport: {
    assetsPerBundle: 10,
  },
  featureFlags: DEFAULT_FEATURE_FLAGS,
} as const;

describe('CLIReporter', () => {
  let originalStdout: any;
  let originalStderr: any;
  let stdoutOutput: any;
  let stderrOutput: any;

  beforeEach(async () => {
    // Stub these out to avoid writing noise to real stdio and to read from these
    // otherwise only writable streams
    originalStdout = process.stdout;
    originalStderr = process.stderr;

    stdoutOutput = '';
    stderrOutput = '';

    let mockStdout = new PassThrough();
    mockStdout.on(
      'data',
      (d: any) => (stdoutOutput += stripAnsi(d.toString())),
    );
    let mockStderr = new PassThrough();
    mockStderr.on(
      'data',
      (d: any) => (stderrOutput += stripAnsi(d.toString())),
    );
    _setStdio(mockStdout, mockStderr);

    await _report(
      {
        type: 'buildStart',
      },
      EMPTY_OPTIONS,
    );
  });

  afterEach(() => {
    _setStdio(originalStdout, originalStderr);
  });

  it('writes log, info, success, and verbose log messages to stdout', async () => {
    let options = {
      ...EMPTY_OPTIONS,
      logLevel: 'verbose',
    };

    await _report(
      {
        type: 'log',
        level: 'info',
        diagnostics: [
          {
            origin: 'test',
            message: 'info',
          },
        ],
      },
      // @ts-expect-error - TS2345 - Argument of type '{ logLevel: string; cacheDir: ".parcel-cache"; parcelVersion: ""; entries: readonly []; targets: readonly []; projectRoot: ""; distDir: "dist"; lockFile: undefined; shouldAutoInstall: false; shouldBuildLazily: false; ... 13 more ...; featureFlags: FeatureFlags; }' is not assignable to parameter of type 'PluginOptions'.
      options,
    );
    // @ts-expect-error - TS2345 - Argument of type '{ logLevel: string; cacheDir: ".parcel-cache"; parcelVersion: ""; entries: readonly []; targets: readonly []; projectRoot: ""; distDir: "dist"; lockFile: undefined; shouldAutoInstall: false; shouldBuildLazily: false; ... 13 more ...; featureFlags: FeatureFlags; }' is not assignable to parameter of type 'PluginOptions'.
    await _report({type: 'log', level: 'success', message: 'success'}, options);
    await _report(
      {
        type: 'log',
        level: 'verbose',
        diagnostics: [
          {
            origin: 'test',
            message: 'verbose',
          },
        ],
      },
      // @ts-expect-error - TS2345 - Argument of type '{ logLevel: string; cacheDir: ".parcel-cache"; parcelVersion: ""; entries: readonly []; targets: readonly []; projectRoot: ""; distDir: "dist"; lockFile: undefined; shouldAutoInstall: false; shouldBuildLazily: false; ... 13 more ...; featureFlags: FeatureFlags; }' is not assignable to parameter of type 'PluginOptions'.
      options,
    );

    assert.equal(stdoutOutput, 'test: info\nsuccess\ntest: verbose\n');
  });

  it('writes errors and warnings to stderr', async () => {
    await _report(
      {
        type: 'log',
        level: 'error',
        diagnostics: [
          {
            origin: 'test',
            message: 'error',
          },
        ],
      },
      EMPTY_OPTIONS,
    );
    await _report(
      {
        type: 'log',
        level: 'warn',
        diagnostics: [
          {
            origin: 'test',
            message: 'warn',
          },
        ],
      },
      EMPTY_OPTIONS,
    );

    assert.equal(stdoutOutput, '\n\n');
    assert.equal(stderrOutput, 'test: error\ntest: warn\n');
  });

  it('prints errors nicely', async () => {
    await _report(
      {
        type: 'log',
        level: 'error',
        diagnostics: [
          {
            origin: 'test',
            message: 'error',
          },
        ],
      },
      EMPTY_OPTIONS,
    );
    await _report(
      {
        type: 'log',
        level: 'warn',
        diagnostics: [
          {
            origin: 'test',
            message: 'warn',
          },
        ],
      },
      EMPTY_OPTIONS,
    );

    assert.equal(stdoutOutput, '\n\n');
    assert(stderrOutput.includes('test: error\n'));
    assert(stderrOutput.includes('test: warn\n'));
  });

  it('writes buildProgress messages to stdout on the default loglevel', async () => {
    await _report({type: 'buildProgress', phase: 'bundling'}, EMPTY_OPTIONS);
    assert.equal(stdoutOutput, 'Bundling...\n');
  });

  it('writes buildSuccess messages to stdout on the default loglevel', async () => {
    await _report({type: 'buildProgress', phase: 'bundling'}, EMPTY_OPTIONS);
    assert.equal(stdoutOutput, 'Bundling...\n');
  });

  it('writes phase timings to stdout when ATLASPACK_SHOW_PHASE_TIMES is set', async () => {
    let oldPhaseTimings = process.env['ATLASPACK_SHOW_PHASE_TIMES'];
    const bundleReportStub = sinon.stub(bundleReport, 'default');
    const persistSpinnerStub = sinon.stub(render, 'persistSpinner');

    after(() => {
      bundleReportStub.restore();
      persistSpinnerStub.restore();
      process.env['ATLASPACK_SHOW_PHASE_TIMES'] = oldPhaseTimings;
    });

    // emit a buildSuccess event to reset the timings and seen phases
    // from the previous test
    process.env['ATLASPACK_SHOW_PHASE_TIMES'] = undefined;
    // @ts-expect-error - TS2345 - Argument of type '{ type: "buildSuccess"; }' is not assignable to parameter of type 'ReporterEvent'.
    await _report({type: 'buildSuccess'}, EMPTY_OPTIONS);

    process.env['ATLASPACK_SHOW_PHASE_TIMES'] = 'true';
    await _report(
      {type: 'buildProgress', phase: 'transforming', filePath: 'foo.js'},
      EMPTY_OPTIONS,
    );
    await _report({type: 'buildProgress', phase: 'bundling'}, EMPTY_OPTIONS);
    await _report(
      // $FlowFixMe
      {
        type: 'buildProgress',
        // @ts-expect-error - TS2322 - Type '"packaging"' is not assignable to type '"optimizing"'.
        phase: 'packaging',
        // @ts-expect-error - TS2740 - Type '{ displayName: string; }' is missing the following properties from type 'NamedBundle': publicId, name, id, type, and 12 more.
        bundle: {
          displayName: 'test',
        },
      },
      EMPTY_OPTIONS,
    );
    // @ts-expect-error - TS2345 - Argument of type '{ type: "buildSuccess"; }' is not assignable to parameter of type 'ReporterEvent'.
    await _report({type: 'buildSuccess'}, EMPTY_OPTIONS);
    const expected =
      /Building...\nBundling...\nPackaging & Optimizing...\nTransforming finished in [0-9]ms\nBundling finished in [0-9]ms\nPackaging & Optimizing finished in [0-9]ms/;

    assert.equal(expected.test(stdoutOutput), true);

    stdoutOutput = '';

    await _report(
      {type: 'buildProgress', phase: 'transforming', filePath: 'foo.js'},
      EMPTY_OPTIONS,
    );
    await _report({type: 'buildProgress', phase: 'bundling'}, EMPTY_OPTIONS);
    await _report(
      // $FlowFixMe
      {
        type: 'buildProgress',
        // @ts-expect-error - TS2322 - Type '"packaging"' is not assignable to type '"optimizing"'.
        phase: 'packaging',
        // @ts-expect-error - TS2740 - Type '{ displayName: string; }' is missing the following properties from type 'NamedBundle': publicId, name, id, type, and 12 more.
        bundle: {
          displayName: 'test',
        },
      },
      EMPTY_OPTIONS,
    );
    // @ts-expect-error - TS2345 - Argument of type '{ type: "buildSuccess"; }' is not assignable to parameter of type 'ReporterEvent'.
    await _report({type: 'buildSuccess'}, EMPTY_OPTIONS);

    assert.equal(
      expected.test(stdoutOutput),
      true,
      'STDOUT output did not match',
    );
  });
});
