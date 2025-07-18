// @flow strict-local

import assert from 'assert';
// $FlowFixMe
import expect from 'expect';
import {createEnvironment} from '../src/Environment';
import {initializeMonitoring} from '../../rust';
import {fromEnvironmentId} from '../src/EnvironmentManager';

describe('Environment', () => {
  it('assigns a default environment with nothing passed', () => {
    assert.deepEqual(fromEnvironmentId(createEnvironment()), {
      id: 'd821e85f6b50315e',
      context: 'browser',
      engines: {
        browsers: ['> 0.25%'],
      },
      includeNodeModules: true,
      outputFormat: 'global',
      isLibrary: false,
      shouldOptimize: false,
      shouldScopeHoist: false,
      sourceMap: undefined,
      loc: undefined,
      sourceType: 'module',
      unstableSingleFileOutput: false,
    });
  });

  it('assigns a node context if a node engine is given', () => {
    assert.deepEqual(
      fromEnvironmentId(createEnvironment({engines: {node: '>= 10.0.0'}})),
      {
        id: '2320af923a717577',
        context: 'node',
        engines: {
          node: '>= 10.0.0',
        },
        includeNodeModules: false,
        outputFormat: 'commonjs',
        isLibrary: false,
        shouldOptimize: false,
        shouldScopeHoist: false,
        sourceMap: undefined,
        loc: undefined,
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
        id: '75603271034eff15',
        context: 'browser',
        engines: {
          browsers: ['last 1 version'],
        },
        includeNodeModules: true,
        outputFormat: 'global',
        isLibrary: false,
        shouldOptimize: false,
        shouldScopeHoist: false,
        sourceMap: undefined,
        loc: undefined,
        sourceType: 'module',
        unstableSingleFileOutput: false,
      },
    );
  });

  it('assigns default engines for node', () => {
    assert.deepEqual(fromEnvironmentId(createEnvironment({context: 'node'})), {
      id: 'e45cc12216f7857d',
      context: 'node',
      engines: {
        node: '>= 8.0.0',
      },
      includeNodeModules: false,
      outputFormat: 'commonjs',
      isLibrary: false,
      shouldOptimize: false,
      shouldScopeHoist: false,
      sourceMap: undefined,
      loc: undefined,
      sourceType: 'module',
      unstableSingleFileOutput: false,
    });
  });

  it('assigns default engines for browsers', () => {
    assert.deepEqual(
      fromEnvironmentId(createEnvironment({context: 'browser'})),
      {
        id: 'd821e85f6b50315e',
        context: 'browser',
        engines: {
          browsers: ['> 0.25%'],
        },
        includeNodeModules: true,
        outputFormat: 'global',
        isLibrary: false,
        shouldOptimize: false,
        shouldScopeHoist: false,
        sourceMap: undefined,
        loc: undefined,
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
    } catch (_err) {
      /* ignore */
    }
    const environment = createEnvironment({});
    expect(fromEnvironmentId(environment).id).toEqual('d821e85f6b50315e');
  });
});
