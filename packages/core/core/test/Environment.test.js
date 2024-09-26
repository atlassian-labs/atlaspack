// @flow strict-local

import assert from 'assert';
// $FlowFixMe
import expect from 'expect';
import {createEnvironment} from '../src/Environment';
import {initializeMonitoring} from '../../rust';

describe('Environment', () => {
  it('assigns a default environment with nothing passed', () => {
    assert.deepEqual(createEnvironment(), {
      id: 'b3520b7bb1354733',
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
    });
  });

  it('assigns a node context if a node engine is given', () => {
    assert.deepEqual(createEnvironment({engines: {node: '>= 10.0.0'}}), {
      id: 'c9c83c954254833b',
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
    });
  });

  it('assigns a browser context if browser engines are given', () => {
    assert.deepEqual(
      createEnvironment({engines: {browsers: ['last 1 version']}}),
      {
        id: '9e3193fe9c7301c3',
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
      },
    );
  });

  it('assigns default engines for node', () => {
    assert.deepEqual(createEnvironment({context: 'node'}), {
      id: '6a2f66a2bf8af810',
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
    });
  });

  it('assigns default engines for browsers', () => {
    assert.deepEqual(createEnvironment({context: 'browser'}), {
      id: 'b3520b7bb1354733',
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
    });
  });
});

describe('createEnvironment', function () {
  it('returns a stable hash', () => {
    initializeMonitoring();
    const environment = createEnvironment({});
    expect(environment.id).toEqual('b3520b7bb1354733');
  });
});
