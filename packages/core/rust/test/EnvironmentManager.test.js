// @flow strict-local

import assert from 'assert';
import {
  getAllEnvironments,
  setAllEnvironments,
  getEnvironment,
  addEnvironment,
} from '..';

describe('EnvironmentManager', () => {
  const environment1 = {
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
  };
  const environment2 = {
    id: '23e9eb4debbdc50e',
    context: 'browser',
    engines: {
      browsers: ['> 0.25%'],
    },
    includeNodeModules: true,
    outputFormat: 'global',
    isLibrary: true,
    shouldOptimize: true,
    shouldScopeHoist: false,
    sourceMap: undefined,
    loc: undefined,
    sourceType: 'module',
    unstableSingleFileOutput: false,
  };

  it('we can add and get environments', () => {
    setAllEnvironments([]);
    addEnvironment(environment1);
    addEnvironment(environment2);
    const storedEnvironment = getEnvironment(environment1.id);
    assert.deepEqual(storedEnvironment, environment1);

    const storedEnvironment2 = getEnvironment(environment2.id);
    assert.deepEqual(storedEnvironment2, environment2);

    const environments = getAllEnvironments();
    assert.deepEqual(environments, [environment1, environment2]);

    setAllEnvironments([]);
    const noEnvironments = getAllEnvironments();
    assert.deepEqual(noEnvironments, []);
  });

  it('we can get all environments', () => {
    setAllEnvironments([]);
    addEnvironment(environment1);
    const environments = getAllEnvironments();
    assert.deepEqual(environments, [environment1]);
  });
});
