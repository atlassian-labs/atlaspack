import assert from 'assert';
import expect from 'expect';
import {createEnvironment} from '../src/Environment';
import {initializeMonitoring} from '@atlaspack/rust';
import {fromEnvironmentId} from '../src/EnvironmentManager';

describe('Environment', () => {
  it('assigns a default environment with nothing passed', () => {
    assert.deepEqual(fromEnvironmentId(createEnvironment()), {
      id: '2a4e8c679386d799',
      context: 'browser',
      engines: {
        browsers: ['> 0.25%'],
      },
      includeNodeModules: true,
      outputFormat: 'global',
      isLibrary: false,
      shouldOptimize: false,
      shouldScopeHoist: false,
      sourceMap: null,
      loc: null,
      sourceType: 'module',
      unstableSingleFileOutput: false,
    });
  });

  it('assigns a node context if a node engine is given', () => {
    assert.deepEqual(
      fromEnvironmentId(createEnvironment({engines: {node: '>= 10.0.0'}})),
      {
        id: '87ff0eb3641001f5',
        context: 'node',
        engines: {
          browsers: null,
          node: '>= 10',
        },
        includeNodeModules: false,
        outputFormat: 'commonjs',
        isLibrary: false,
        shouldOptimize: false,
        shouldScopeHoist: false,
        sourceMap: null,
        loc: null,
        sourceType: 'module',
        unstableSingleFileOutput: false,
      },
    );
  });

  it('assigns a browser context if browser engines are given', () => {
    assert.deepEqual(
      fromEnvironmentId(
        createEnvironment({engines: {browsers: ['last 1 version']}}),
      ),
      {
        id: '1b943a7cc24b4334',
        context: 'browser',
        engines: {
          browsers: ['last 1 version'],
        },
        includeNodeModules: true,
        outputFormat: 'global',
        isLibrary: false,
        shouldOptimize: false,
        shouldScopeHoist: false,
        sourceMap: null,
        loc: null,
        sourceType: 'module',
        unstableSingleFileOutput: false,
      },
    );
  });

  it('assigns default engines for node', () => {
    assert.deepEqual(fromEnvironmentId(createEnvironment({context: 'node'})), {
      id: '9d9c5a45c8c3a5a0',
      context: 'node',
      engines: {
        browsers: null,
        node: '>= 8',
      },
      includeNodeModules: false,
      outputFormat: 'commonjs',
      isLibrary: false,
      shouldOptimize: false,
      shouldScopeHoist: false,
      sourceMap: null,
      loc: null,
      sourceType: 'module',
      unstableSingleFileOutput: false,
    });
  });

  it('assigns default engines for browsers', () => {
    assert.deepEqual(
      fromEnvironmentId(createEnvironment({context: 'browser'})),
      {
        id: '2a4e8c679386d799',
        context: 'browser',
        engines: {
          browsers: ['> 0.25%'],
        },
        includeNodeModules: true,
        outputFormat: 'global',
        isLibrary: false,
        shouldOptimize: false,
        shouldScopeHoist: false,
        sourceMap: null,
        loc: null,
        sourceType: 'module',
        unstableSingleFileOutput: false,
      },
    );
  });

  it('assigns default engines for tesseract (same as web-worker)', () => {
    assert.deepEqual(
      fromEnvironmentId(createEnvironment({context: 'tesseract'})),
      {
        id: '70f783b449e9d655',
        context: 'tesseract',
        engines: {
          browsers: ['> 0.25%'],
        },
        includeNodeModules: true,
        outputFormat: 'global',
        isLibrary: false,
        shouldOptimize: false,
        shouldScopeHoist: false,
        sourceMap: null,
        loc: null,
        sourceType: 'module',
        unstableSingleFileOutput: false,
      },
    );
  });
});

describe('createEnvironment', function () {
  it('returns a stable hash', () => {
    try {
      initializeMonitoring();
    } catch (_err: any) {
      /* ignore */
    }
    const environment = createEnvironment({});
    expect(fromEnvironmentId(environment).id).toEqual('2a4e8c679386d799');
  });
});
