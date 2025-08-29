import assert from 'assert';
import expect from 'expect';
import {createEnvironment} from '../src/Environment';
import {initializeMonitoring} from '@atlaspack/rust';
import {fromEnvironmentId} from '../src/EnvironmentManager';

describe('Environment', () => {
  it('assigns a default environment with nothing passed', () => {
    assert.deepEqual(fromEnvironmentId(createEnvironment()), {
      id: 'fe24c9f18fc84924',
      context: 'browser',
      customEnv: null,
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
        id: 'f259a84041f6e6c1',
        context: 'node',
        customEnv: null,
        engines: {
          node: '>= 10.0.0',
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
        id: '24f9769a698269e1',
        context: 'browser',
        customEnv: null,
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
      id: '595069cdd6a2b052',
      context: 'node',
      customEnv: null,
      engines: {
        node: '>= 8.0.0',
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
        id: 'fe24c9f18fc84924',
        context: 'browser',
        customEnv: null,
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
        id: '8cd953811a9f0de3',
        context: 'tesseract',
        customEnv: null,
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
    expect(fromEnvironmentId(environment).id).toEqual('fe24c9f18fc84924');
  });
});
