import assert from 'assert';
import {createEnvironment} from '../../core/src/Environment';
import {fromEnvironmentId} from '../../core/src/EnvironmentManager';
import {
  getAllEnvironments,
  setAllEnvironments,
  getEnvironment,
  addEnvironment,
} from '..';

describe('EnvironmentManager', () => {
  const environment1 = fromEnvironmentId(
    createEnvironment({
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
      customEnv: {
        MY_ENV: 'one',
      },
    }),
  );
  const environment2 = fromEnvironmentId(
    createEnvironment({
      context: 'browser',
      engines: {
        browsers: ['> 0.25%'],
      },
      includeNodeModules: true,
      outputFormat: 'global',
      isLibrary: true,
      shouldOptimize: true,
      shouldScopeHoist: false,
      sourceMap: null,
      loc: null,
      sourceType: 'module',
      unstableSingleFileOutput: false,
      customEnv: {
        MY_ENV: 'two',
      },
    }),
  );

  it('we can add and get environments', () => {
    setAllEnvironments([]);
    addEnvironment(environment1);
    addEnvironment(environment2);
    const storedEnvironment = getEnvironment(environment1.id);
    assert.deepEqual(storedEnvironment, environment1);

    const storedEnvironment2 = getEnvironment(environment2.id);
    assert.deepEqual(storedEnvironment2, environment2);

    const environments = getAllEnvironments();
    environments.sort((a: any, b: any) => a.id.localeCompare(b.id));
    assert.deepEqual(
      environments.sort((a: any, b: any) => a.id.localeCompare(b.id)),
      [environment2, environment1].sort((a: any, b: any) =>
        a.id.localeCompare(b.id),
      ),
    );

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
