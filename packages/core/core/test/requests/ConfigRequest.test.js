// @flow strict-local

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
import {getObjectKey, runConfigRequest} from '../../src/requests/ConfigRequest';
import {toProjectPath} from '../../src/projectPath';

// $FlowFixMe unclear-type forgive me
const mockCast = (f: any): any => f;

describe('ConfigRequest tests', () => {
  const projectRoot = 'project_root';
  const farm = new WorkerFarm({
    workerPath: require.resolve('../../src/worker'),
    maxConcurrentWorkers: 1,
  });
  let fs = new MemoryFS(farm);
  beforeEach(() => {
    fs = new MemoryFS(farm);
  });

  const getMockRunApi = (
    options: mixed = {projectRoot, inputFS: fs},
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
      runRequest: sinon.spy((request) => {
        return request.run({
          api: mockRunApi,
          options,
        });
      }),
    };
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
      ['config.json', 'key1', hashString('"value1"')],
      'Invalidate was called for key1',
    );
  });
});

describe('getObjectKey', () => {
  it('can get a key from an object', () => {
    const obj = {a: {b: {c: 'd'}}};
    assert.equal(getObjectKey(obj, ['a', 'b', 'c']), 'd');
  });

  it('returns the original object when key array is empty', () => {
    const obj = {a: 1, b: 2};
    assert.deepEqual(getObjectKey(obj, []), obj);
  });

  it('can access single-level properties', () => {
    const obj = {name: 'test', age: 25};
    assert.equal(getObjectKey(obj, ['name']), 'test');
    assert.equal(getObjectKey(obj, ['age']), 25);
  });

  it('returns undefined for non-existent keys', () => {
    const obj = {a: {b: 'value'}};
    assert.equal(getObjectKey(obj, ['nonexistent']), undefined);
    assert.equal(getObjectKey(obj, ['a', 'nonexistent']), undefined);
    assert.equal(getObjectKey(obj, ['a', 'b', 'nonexistent']), undefined);
  });

  it('handles null and undefined values in the path', () => {
    const obj = {a: null, b: {c: undefined}};
    assert.equal(getObjectKey(obj, ['a']), null);
    assert.equal(getObjectKey(obj, ['b', 'c']), undefined);
  });

  it('throws when trying to access property of null', () => {
    const obj = {a: null};
    assert.throws(() => {
      getObjectKey(obj, ['a', 'b']);
    }, TypeError);
  });

  it('throws when trying to access property of undefined', () => {
    const obj = {a: undefined};
    assert.throws(() => {
      getObjectKey(obj, ['a', 'b']);
    }, TypeError);
  });

  it('can access array elements', () => {
    const obj = {arr: ['first', 'second', 'third']};
    assert.equal(getObjectKey(obj, ['arr', '0']), 'first');
    assert.equal(getObjectKey(obj, ['arr', '1']), 'second');
    assert.equal(getObjectKey(obj, ['arr', '2']), 'third');
  });

  it('can access nested arrays and objects', () => {
    const obj = {
      data: [
        {name: 'item1', props: {color: 'red'}},
        {name: 'item2', props: {color: 'blue'}},
      ],
    };
    assert.equal(getObjectKey(obj, ['data', '0', 'name']), 'item1');
    assert.equal(getObjectKey(obj, ['data', '1', 'props', 'color']), 'blue');
  });

  it('handles numeric keys as strings', () => {
    const obj = {'0': 'first', '1': {nested: 'value'}};
    assert.equal(getObjectKey(obj, ['0']), 'first');
    assert.equal(getObjectKey(obj, ['1', 'nested']), 'value');
  });

  it('handles keys with special characters', () => {
    const obj = {
      'key-with-dashes': 'value1',
      'key.with.dots': {
        'nested-key': 'value2',
      },
      'key with spaces': 'value3',
      '@special$chars#': 'value4',
    };
    assert.equal(getObjectKey(obj, ['key-with-dashes']), 'value1');
    assert.equal(getObjectKey(obj, ['key.with.dots', 'nested-key']), 'value2');
    assert.equal(getObjectKey(obj, ['key with spaces']), 'value3');
    assert.equal(getObjectKey(obj, ['@special$chars#']), 'value4');
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
    };
    assert.equal(getObjectKey(obj, ['zero']), 0);
    assert.equal(getObjectKey(obj, ['false']), false);
    assert.equal(getObjectKey(obj, ['emptyString']), '');
    assert.equal(getObjectKey(obj, ['nullValue']), null);
    assert.equal(getObjectKey(obj, ['undefinedValue']), undefined);
    assert.equal(getObjectKey(obj, ['nested', 'zero']), 0);
    assert.equal(getObjectKey(obj, ['nested', 'false']), false);
  });

  it('can access function values', () => {
    const testFunc = () => 'test';
    const obj = {
      func: testFunc,
      nested: {
        method: function () {
          return 'method';
        },
      },
    };
    assert.equal(getObjectKey(obj, ['func']), testFunc);
    assert.equal(typeof getObjectKey(obj, ['nested', 'method']), 'function');
    assert.equal(getObjectKey(obj, ['nested', 'method'])(), 'method');
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
    };
    assert.equal(
      getObjectKey(obj, [
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

  it('handles objects with prototype properties', () => {
    const TestConstructor = function () {
      // $FlowFixMe
      this.own = 'ownProperty';
    };
    TestConstructor.prototype.inherited = 'inheritedProperty';

    // $FlowFixMe
    const obj = new TestConstructor();
    assert.equal(getObjectKey(obj, ['own']), 'ownProperty');
    assert.equal(getObjectKey(obj, ['inherited']), 'inheritedProperty');
  });

  it('handles Date objects', () => {
    const date = new Date('2023-01-01');
    const obj = {
      timestamp: date,
      nested: {
        date: date,
      },
    };
    assert.equal(getObjectKey(obj, ['timestamp']), date);
    assert.equal(getObjectKey(obj, ['nested', 'date']), date);
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
    };

    assert.equal(
      getObjectKey(obj, ['users', '0', 'profile', 'settings', 'theme']),
      'dark',
    );
    assert.equal(
      getObjectKey(obj, ['users', '1', 'profile', 'settings', 'notifications']),
      false,
    );
    assert.equal(getObjectKey(obj, ['config', 'features', '0']), 'feature1');
  });

  it('throws when object is null', () => {
    assert.throws(() => {
      getObjectKey(null, ['key']);
    }, TypeError);
  });

  it('throws when object is undefined', () => {
    assert.throws(() => {
      getObjectKey(undefined, ['key']);
    }, TypeError);
  });
});
