import WorkerFarm from '@atlaspack/workers';
import path from 'path';
import assert from 'assert';
import sinon from 'sinon';
import {MemoryFS} from '@atlaspack/fs';
import {hashString} from '@atlaspack/rust';

import type {
  ConfigRequest,
  ConfigRequestResult,
} from '../../src/requests/ConfigRequest';
import type {RunAPI} from '../../src/RequestTracker';
import {
  getValueAtPath,
  runConfigRequest,
} from '../../src/requests/ConfigRequest';
import {toProjectPath} from '../../src/projectPath';

const mockCast = (f: any): any => f;

describe('ConfigRequest tests', () => {
  const projectRoot = 'project_root';
  let farm: any;
  let fs: any;
  before(() => {
    farm = new WorkerFarm({
      workerPath: require.resolve('../../src/worker'),
      maxConcurrentWorkers: 1,
    });
  });

  beforeEach(() => {
    fs = new MemoryFS(farm);
  });

  after(() => {
    farm.end();
  });

  const getMockRunApi = (
    options: unknown = {projectRoot, inputFS: fs},
  ): RunAPI<ConfigRequestResult> => {
    const mockRunApi = {
      storeResult: sinon.spy(),
      canSkipSubrequest: sinon.spy(),
      invalidateOnFileCreate: sinon.spy(),
      getInvalidSubRequests: sinon.spy(),
      getInvalidations: sinon.spy(),
      getPreviousResult: sinon.spy(),
      getRequestResult: sinon.spy(),
      getSubRequests: sinon.spy(),
      invalidateOnBuild: sinon.spy(),
      invalidateOnConfigKeyChange: sinon.spy(),
      invalidateOnEnvChange: sinon.spy(),
      invalidateOnFileDelete: sinon.spy(),
      invalidateOnFileUpdate: sinon.spy(),
      invalidateOnOptionChange: sinon.spy(),
      invalidateOnStartup: sinon.spy(),
      runRequest: sinon.spy((request: any) => {
        return request.run({
          api: mockRunApi,
          options,
        });
      }),
    } as const;
    return mockRunApi;
  };

  const baseRequest: ConfigRequest = {
    id: 'config_request_test',
    invalidateOnBuild: false,
    invalidateOnConfigKeyChange: [],
    invalidateOnFileCreate: [],
    invalidateOnEnvChange: new Set(),
    invalidateOnOptionChange: new Set(),
    invalidateOnStartup: false,
    invalidateOnFileChange: new Set(),
  };

  it('can execute a config request', async () => {
    const mockRunApi = getMockRunApi();
    await runConfigRequest(mockRunApi, {
      ...baseRequest,
    });
  });

  it('forwards "invalidateOnFileChange" calls to runAPI', async () => {
    const mockRunApi = getMockRunApi();
    await runConfigRequest(mockRunApi, {
      ...baseRequest,
      invalidateOnFileChange: new Set([
        toProjectPath(projectRoot, 'path1'),
        toProjectPath(projectRoot, 'path2'),
      ]),
    });

    assert(
      mockCast(mockRunApi.invalidateOnFileUpdate).called,
      'Invalidate was called',
    );
    assert(
      mockCast(mockRunApi.invalidateOnFileUpdate).calledWith('path1'),
      'Invalidate was called with path1',
    );
    assert(
      mockCast(mockRunApi.invalidateOnFileUpdate).calledWith('path2'),
      'Invalidate was called with path2',
    );
    assert(
      mockCast(mockRunApi.invalidateOnFileDelete).calledWith('path1'),
      'Invalidate was called with path1',
    );
    assert(
      mockCast(mockRunApi.invalidateOnFileDelete).calledWith('path2'),
      'Invalidate was called with path2',
    );
  });

  it('forwards "invalidateOnFileCreate" calls to runAPI', async () => {
    const mockRunApi = getMockRunApi();
    await runConfigRequest(mockRunApi, {
      ...baseRequest,
      invalidateOnFileCreate: [
        {filePath: toProjectPath(projectRoot, 'filePath')},
        {glob: toProjectPath(projectRoot, 'glob')},
        {
          fileName: 'package.json',
          aboveFilePath: toProjectPath(projectRoot, 'fileAbove'),
        },
      ],
    });

    assert(
      mockCast(mockRunApi.invalidateOnFileCreate).called,
      'Invalidate was called',
    );
    assert(
      mockCast(mockRunApi.invalidateOnFileCreate).calledWithMatch({
        filePath: 'filePath',
      }),
      'Invalidate was called for path',
    );
    assert(
      mockCast(mockRunApi.invalidateOnFileCreate).calledWithMatch({
        glob: 'glob',
      }),
      'Invalidate was called for glob',
    );
    assert(
      mockCast(mockRunApi.invalidateOnFileCreate).calledWithMatch({
        fileName: 'package.json',
        aboveFilePath: 'fileAbove',
      }),
      'Invalidate was called for fileAbove',
    );
  });

  it('forwards "invalidateOnEnvChange" calls to runAPI', async () => {
    const mockRunApi = getMockRunApi();
    await runConfigRequest(mockRunApi, {
      ...baseRequest,
      invalidateOnEnvChange: new Set(['env1', 'env2']),
    });

    assert(
      mockCast(mockRunApi.invalidateOnEnvChange).called,
      'Invalidate was called',
    );
    assert(
      mockCast(mockRunApi.invalidateOnEnvChange).calledWithMatch('env1'),
      'Invalidate was called for env1',
    );
    assert(
      mockCast(mockRunApi.invalidateOnEnvChange).calledWithMatch('env2'),
      'Invalidate was called for env1',
    );
  });

  it('forwards "invalidateOnOptionChange" calls to runAPI', async () => {
    const mockRunApi = getMockRunApi();
    await runConfigRequest(mockRunApi, {
      ...baseRequest,
      invalidateOnOptionChange: new Set(['option1', 'option2']),
    });

    assert(
      mockCast(mockRunApi.invalidateOnOptionChange).called,
      'Invalidate was called',
    );
    assert(
      mockCast(mockRunApi.invalidateOnOptionChange).calledWithMatch('option1'),
      'Invalidate was called for option1',
    );
    assert(
      mockCast(mockRunApi.invalidateOnOptionChange).calledWithMatch('option2'),
      'Invalidate was called for option2',
    );
  });

  it('forwards "invalidateOnStartup" calls to runAPI', async () => {
    const mockRunApi = getMockRunApi();
    await runConfigRequest(mockRunApi, {
      ...baseRequest,
      invalidateOnStartup: true,
    });

    assert(
      mockCast(mockRunApi.invalidateOnStartup).called,
      'Invalidate was called',
    );
  });

  it('forwards "invalidateOnBuild" calls to runAPI', async () => {
    const mockRunApi = getMockRunApi();
    await runConfigRequest(mockRunApi, {
      ...baseRequest,
      invalidateOnBuild: true,
    });

    assert(
      mockCast(mockRunApi.invalidateOnBuild).called,
      'Invalidate was called',
    );
  });

  it('forwards "invalidateOnConfigKeyChange" calls to runAPI', async () => {
    await fs.mkdirp('/project_root');
    await fs.writeFile(
      '/project_root/config.json',
      JSON.stringify({key1: 'value1'}),
    );
    sinon.spy(fs, 'readFile');
    sinon.spy(fs, 'readFileSync');
    const mockRunApi = getMockRunApi();
    await runConfigRequest(mockRunApi, {
      ...baseRequest,
      invalidateOnConfigKeyChange: [
        {
          configKey: ['key1'],
          filePath: toProjectPath(projectRoot, 'config.json'),
        },
      ],
    });

    const fsCall = mockCast(fs).readFile.getCall(0);
    assert.deepEqual(
      fsCall?.args,
      [path.join('project_root', 'config.json'), 'utf8'],
      'readFile was called',
    );

    const call = mockCast(mockRunApi.invalidateOnConfigKeyChange).getCall(0);
    assert.deepEqual(
      call.args,
      ['config.json', ['key1'], hashString('"value1"')],
      'Invalidate was called for key1',
    );
  });
});

describe('getValueAtPath', () => {
  it('can get a key from an object', () => {
    const obj = {a: {b: {c: 'd'}}} as const;
    assert.equal(getValueAtPath(obj, ['a', 'b', 'c']), 'd');
  });

  it('returns the original object when key array is empty', () => {
    const obj = {a: 1, b: 2} as const;
    assert.deepEqual(getValueAtPath(obj, []), obj);
  });

  it('can access single-level properties', () => {
    const obj = {name: 'test', age: 25} as const;
    assert.equal(getValueAtPath(obj, ['name']), 'test');
    assert.equal(getValueAtPath(obj, ['age']), 25);
  });

  it('returns undefined for non-existent keys', () => {
    const obj = {a: {b: 'value'}} as const;
    assert.equal(getValueAtPath(obj, ['nonexistent']), undefined);
    assert.equal(getValueAtPath(obj, ['a', 'nonexistent']), undefined);
    assert.equal(getValueAtPath(obj, ['a', 'b', 'nonexistent']), undefined);
  });

  it('handles null and undefined values in the path', () => {
    const obj = {a: null, b: {c: undefined}} as const;
    assert.equal(getValueAtPath(obj, ['a']), null);
    assert.equal(getValueAtPath(obj, ['b', 'c']), undefined);
  });

  it('does not throw when trying to access property of null', () => {
    const obj = {a: null} as const;
    assert.equal(getValueAtPath(obj, ['a', 'b']), undefined);
  });

  it('does not throw when trying to access property of undefined', () => {
    const obj = {a: undefined} as const;
    assert.equal(getValueAtPath(obj, ['a', 'b']), undefined);
  });

  it('can access nested arrays and objects', () => {
    const obj = {
      data: [
        {name: 'item1', props: {color: 'red'}},
        {name: 'item2', props: {color: 'blue'}},
      ],
    } as const;
    assert.equal(getValueAtPath(obj, ['data', '0', 'name']), 'item1');
    assert.equal(getValueAtPath(obj, ['data', '1', 'props', 'color']), 'blue');
  });

  it('handles numeric keys as strings', () => {
    const obj = {'0': 'first', '1': {nested: 'value'}} as const;
    assert.equal(getValueAtPath(obj, ['0']), 'first');
    assert.equal(getValueAtPath(obj, ['1', 'nested']), 'value');
  });

  it('handles keys with special characters', () => {
    const obj = {
      'key-with-dashes': 'value1',
      'key.with.dots': {
        'nested-key': 'value2',
      },
      'key with spaces': 'value3',
      '@special$chars#': 'value4',
    } as const;
    assert.equal(getValueAtPath(obj, ['key-with-dashes']), 'value1');
    assert.equal(
      getValueAtPath(obj, ['key.with.dots', 'nested-key']),
      'value2',
    );
    assert.equal(getValueAtPath(obj, ['key with spaces']), 'value3');
    assert.equal(getValueAtPath(obj, ['@special$chars#']), 'value4');
  });

  it('handles falsy values correctly', () => {
    const obj = {
      zero: 0,
      false: false,
      emptyString: '',
      nullValue: null,
      undefinedValue: undefined,
      nested: {
        zero: 0,
        false: false,
      },
    } as const;
    assert.equal(getValueAtPath(obj, ['zero']), 0);
    assert.equal(getValueAtPath(obj, ['false']), false);
    assert.equal(getValueAtPath(obj, ['emptyString']), '');
    assert.equal(getValueAtPath(obj, ['nullValue']), null);
    assert.equal(getValueAtPath(obj, ['undefinedValue']), undefined);
    assert.equal(getValueAtPath(obj, ['nested', 'zero']), 0);
    assert.equal(getValueAtPath(obj, ['nested', 'false']), false);
  });

  it('handles deep nesting', () => {
    const obj = {
      level1: {
        level2: {
          level3: {
            level4: {
              level5: {
                deepValue: 'found',
              },
            },
          },
        },
      },
    } as const;
    assert.equal(
      getValueAtPath(obj, [
        'level1',
        'level2',
        'level3',
        'level4',
        'level5',
        'deepValue',
      ]),
      'found',
    );
  });

  it('handles Date objects', () => {
    const date = new Date('2023-01-01');
    const obj = {
      timestamp: date,
      nested: {
        date: date,
      },
    } as const;
    assert.equal(getValueAtPath(obj, ['timestamp']), date);
    assert.equal(getValueAtPath(obj, ['nested', 'date']), date);
  });

  it('handles complex nested structures with mixed types', () => {
    const obj = {
      users: [
        {
          id: 1,
          profile: {
            settings: {
              theme: 'dark',
              notifications: true,
            },
          },
        },
        {
          id: 2,
          profile: {
            settings: {
              theme: 'light',
              notifications: false,
            },
          },
        },
      ],
      config: {
        version: '1.0.0',
        features: ['feature1', 'feature2'],
      },
    } as const;

    assert.equal(
      getValueAtPath(obj, ['users', '0', 'profile', 'settings', 'theme']),
      'dark',
    );
    assert.equal(
      getValueAtPath(obj, [
        'users',
        '1',
        'profile',
        'settings',
        'notifications',
      ]),
      false,
    );
    assert.equal(getValueAtPath(obj, ['config', 'features', '0']), 'feature1');
  });
});
