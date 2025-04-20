// @flow
import assert from 'assert';
import path from 'path';

import nullthrows from 'nullthrows';

import {AtlaspackConfig} from '../src/AtlaspackConfig';
import {toProjectPath} from '../src/projectPath';
import {
  validateConfigFile,
  mergePipelines,
  mergeMaps,
  mergeConfigs,
  resolveExtends,
  parseAndProcessConfig,
  resolveAtlaspackConfig,
  processConfig,
} from '../src/requests/AtlaspackConfigRequest';

import {DEFAULT_OPTIONS, relative} from './test-utils';

describe('AtlaspackConfigRequest', () => {
  describe('validateConfigFile', () => {
    it('should require pipeline to be an array', () => {
      assert.throws(() => {
        validateConfigFile(
          // $FlowExpectedError[incompatible-call]
          {
            filePath: '.parcelrc',
            resolvers: '123',
          },
          '.parcelrc',
        );
      });
    });

    it('should require pipeline elements to be strings', () => {
      assert.throws(() => {
        validateConfigFile(
          {
            filePath: '.parcelrc',
            // $FlowExpectedError[incompatible-call]
            resolvers: [1, '123', 5],
          },
          '.parcelrc',
        );
      });
    });

    it('should succeed with an array of valid package names', () => {
      validateConfigFile(
        {
          filePath: '.parcelrc',
          resolvers: ['parcel-resolver-test'],
        },
        '.parcelrc',
      );
    });

    it('should support spread elements', () => {
      validateConfigFile(
        {
          filePath: '.parcelrc',
          resolvers: ['parcel-resolver-test', '...'],
        },
        '.parcelrc',
      );
    });

    it('should require glob map to be an object', () => {
      assert.throws(() => {
        validateConfigFile(
          {
            filePath: '.parcelrc',
            // $FlowExpectedError[incompatible-call]
            transformers: ['parcel-transformer-test', '...'],
          },
          '.parcelrc',
        );
      });
    });

    it('should require extends to be a string or array of strings', () => {
      assert.throws(() => {
        validateConfigFile(
          // $FlowExpectedError[incompatible-call]
          {
            filePath: '.parcelrc',
            extends: 2,
          },
          '.parcelrc',
        );
      });

      assert.throws(() => {
        validateConfigFile(
          {
            filePath: '.parcelrc',
            // $FlowExpectedError[incompatible-call]
            extends: [2, 7],
          },
          '.parcelrc',
        );
      });
    });

    it('should support relative paths', () => {
      validateConfigFile(
        {
          filePath: '.parcelrc',
          extends: './foo',
        },
        '.parcelrc',
      );

      validateConfigFile(
        {
          filePath: '.parcelrc',
          extends: ['./foo', './bar'],
        },
        '.parcelrc',
      );
    });

    it('should throw for invalid top level keys', () => {
      assert.throws(
        () => {
          validateConfigFile(
            // $FlowExpectedError
            {
              extends: '@atlaspack/config-default',
              '@atlaspack/transformer-js': {
                inlineEnvironment: false,
              },
            },
            '.parcelrc',
          );
        },
        (e) => {
          assert.strictEqual(
            e.diagnostics[0].codeFrames[0].codeHighlights[0].message,
            `Possible values: "$schema", "bundler", "resolvers", "transformers", "validators", "namers", "packagers", "optimizers", "compressors", "reporters", "runtimes", "filePath", "resolveFrom"`,
          );
          return true;
        },
      );
    });

    it('should succeed on valid config', () => {
      validateConfigFile(
        {
          filePath: '.parcelrc',
          extends: 'parcel-config-foo',
          transformers: {
            '*.js': ['parcel-transformer-foo'],
          },
        },
        '.parcelrc',
      );
    });

    it('should throw error on empty config file', () => {
      assert.throws(
        () => {
          validateConfigFile({}, '.parcelrc');
        },
        {name: 'Error', message: ".parcelrc can't be empty"},
      );
    });
  });

  describe('mergePipelines', () => {
    it('should return an empty array if base and extension are null', () => {
      assert.deepEqual(mergePipelines(null, null), []);
    });

    it('should return base if extension is null', () => {
      assert.deepEqual(
        mergePipelines(
          [
            {
              packageName: 'parcel-transform-foo',
              resolveFrom: toProjectPath('/', '/.parcelrc'),
              keyPath: '/transformers/*.js/0',
            },
          ],
          null,
        ),
        [
          {
            packageName: 'parcel-transform-foo',
            resolveFrom: '.parcelrc',
            keyPath: '/transformers/*.js/0',
          },
        ],
      );
    });

    it('should return extension if base is null', () => {
      assert.deepEqual(
        mergePipelines(null, [
          {
            packageName: 'parcel-transform-bar',
            resolveFrom: toProjectPath('/', '/.parcelrc'),
            keyPath: '/transformers/*.js/0',
          },
        ]),
        [
          {
            packageName: 'parcel-transform-bar',
            resolveFrom: '.parcelrc',
            keyPath: '/transformers/*.js/0',
          },
        ],
      );
    });

    it('should return extension if there are no spread elements', () => {
      assert.deepEqual(
        mergePipelines(
          [
            {
              packageName: 'parcel-transform-foo',
              resolveFrom: toProjectPath('/', '/.parcelrc'),
              keyPath: '/transformers/*.js/0',
            },
          ],
          [
            {
              packageName: 'parcel-transform-bar',
              resolveFrom: toProjectPath('/', '/.parcelrc'),
              keyPath: '/transformers/*.js/0',
            },
          ],
        ),
        [
          {
            packageName: 'parcel-transform-bar',
            resolveFrom: '.parcelrc',
            keyPath: '/transformers/*.js/0',
          },
        ],
      );
    });

    it('should return merge base into extension if there are spread elements', () => {
      assert.deepEqual(
        mergePipelines(
          [
            {
              packageName: 'parcel-transform-foo',
              resolveFrom: toProjectPath('/', '/.parcelrc'),
              keyPath: '/transformers/*.js/0',
            },
          ],
          [
            {
              packageName: 'parcel-transform-bar',
              resolveFrom: toProjectPath('/', '/.parcelrc'),
              keyPath: '/transformers/*.js/0',
            },
            '...',
            {
              packageName: 'parcel-transform-baz',
              resolveFrom: toProjectPath('/', '/.parcelrc'),
              keyPath: '/transformers/*.js/2',
            },
          ],
        ),
        [
          {
            packageName: 'parcel-transform-bar',
            resolveFrom: '.parcelrc',
            keyPath: '/transformers/*.js/0',
          },
          {
            packageName: 'parcel-transform-foo',
            resolveFrom: '.parcelrc',
            keyPath: '/transformers/*.js/0',
          },
          {
            packageName: 'parcel-transform-baz',
            resolveFrom: '.parcelrc',
            keyPath: '/transformers/*.js/2',
          },
        ],
      );
    });

    it('should throw if more than one spread element is in a pipeline', () => {
      assert.throws(() => {
        mergePipelines(
          [
            {
              packageName: 'parcel-transform-foo',
              resolveFrom: toProjectPath('/', '/.parcelrc'),
              keyPath: '/transformers/*.js/0',
            },
          ],
          [
            {
              packageName: 'parcel-transform-bar',
              resolveFrom: toProjectPath('/', '/.parcelrc'),
              keyPath: '/transformers/*.js/0',
            },
            '...',
            {
              packageName: 'parcel-transform-baz',
              resolveFrom: toProjectPath('/', '/.parcelrc'),
              keyPath: '/transformers/*.js/2',
            },
            '...',
          ],
        );
      }, /Only one spread element can be included in a config pipeline/);
    });

    it('should remove spread element even without a base map', () => {
      assert.deepEqual(
        mergePipelines(null, [
          {
            packageName: 'parcel-transform-bar',
            resolveFrom: toProjectPath('/', '/.parcelrc'),
            keyPath: '/transformers/*.js/0',
          },
          '...',
          {
            packageName: 'parcel-transform-baz',
            resolveFrom: toProjectPath('/', '/.parcelrc'),
            keyPath: '/transformers/*.js/2',
          },
        ]),
        [
          {
            packageName: 'parcel-transform-bar',
            resolveFrom: '.parcelrc',
            keyPath: '/transformers/*.js/0',
          },
          {
            packageName: 'parcel-transform-baz',
            resolveFrom: '.parcelrc',
            keyPath: '/transformers/*.js/2',
          },
        ],
      );
    });

    it('should throw if more than one spread element is in a pipeline even without a base map', () => {
      assert.throws(() => {
        mergePipelines(null, [
          {
            packageName: 'parcel-transform-bar',
            resolveFrom: toProjectPath('/', '/.parcelrc'),
            keyPath: '/transformers/*.js/0',
          },
          '...',
          {
            packageName: 'parcel-transform-baz',
            resolveFrom: toProjectPath('/', '/.parcelrc'),
            keyPath: '/transformers/*.js/2',
          },
          '...',
        ]);
      }, /Only one spread element can be included in a config pipeline/);
    });
  });

  describe('mergeMaps', () => {
    it('should return an empty object if base and extension are null', () => {
      assert.deepEqual(mergeMaps(null, null), {});
    });

    it('should return base if extension is null', () => {
      assert.deepEqual(mergeMaps({'*.js': 'foo'}, null), {
        '*.js': 'foo',
      });
    });

    it('should return extension if base is null', () => {
      assert.deepEqual(mergeMaps(null, {'*.js': 'foo'}), {
        '*.js': 'foo',
      });
    });

    it('should merge the objects', () => {
      assert.deepEqual(
        mergeMaps({'*.css': 'css', '*.js': 'base-js'}, {'*.js': 'ext-js'}),
        {'*.js': 'ext-js', '*.css': 'css'},
      );
    });

    it('should ensure that extension properties have a higher precedence than base properties', () => {
      let merged = mergeMaps({'*.{js,jsx}': 'base-js'}, {'*.js': 'ext-js'});
      assert.deepEqual(merged, {'*.js': 'ext-js', '*.{js,jsx}': 'base-js'});
      assert.deepEqual(Object.keys(merged), ['*.js', '*.{js,jsx}']);
    });

    it('should call a merger function if provided', () => {
      let merger = (a, b) => [a, b];
      assert.deepEqual(
        mergeMaps({'*.js': 'base-js'}, {'*.js': 'ext-js'}, merger),
        {'*.js': ['base-js', 'ext-js']},
      );
    });
  });

  describe('mergeConfigs', () => {
    it('should merge configs', () => {
      let base = new AtlaspackConfig(
        {
          filePath: toProjectPath('/', '/.parcelrc'),
          resolvers: [
            {
              packageName: 'parcel-resolver-base',
              resolveFrom: toProjectPath('/', '/.parcelrc'),
              keyPath: '/resolvers/0',
            },
          ],
          transformers: {
            '*.js': [
              {
                packageName: 'parcel-transform-base',
                resolveFrom: toProjectPath('/', '/.parcelrc'),
                keyPath: '/transformers/*.js/0',
              },
            ],
            '*.css': [
              {
                packageName: 'parcel-transform-css',
                resolveFrom: toProjectPath('/', '/.parcelrc'),
                keyPath: '/transformers/*.css/0',
              },
            ],
          },
          bundler: {
            packageName: 'parcel-bundler-base',
            resolveFrom: toProjectPath('/', '/.parcelrc'),
            keyPath: '/bundler',
          },
        },
        DEFAULT_OPTIONS,
      );

      let ext = {
        filePath: '.parcelrc',
        resolvers: [
          {
            packageName: 'parcel-resolver-ext',
            resolveFrom: '.parcelrc',
            keyPath: '/resolvers/0',
          },
          '...',
        ],
        transformers: {
          '*.js': [
            {
              packageName: 'parcel-transform-ext',
              resolveFrom: '.parcelrc',
              keyPath: '/transformers/*.js/0',
            },
            '...',
          ],
        },
      };

      let merged = {
        filePath: '.parcelrc',
        resolvers: [
          {
            packageName: 'parcel-resolver-ext',
            resolveFrom: '.parcelrc',
            keyPath: '/resolvers/0',
          },
          {
            packageName: 'parcel-resolver-base',
            resolveFrom: '.parcelrc',
            keyPath: '/resolvers/0',
          },
        ],
        transformers: {
          '*.js': [
            {
              packageName: 'parcel-transform-ext',
              resolveFrom: '.parcelrc',
              keyPath: '/transformers/*.js/0',
            },
            {
              packageName: 'parcel-transform-base',
              resolveFrom: '.parcelrc',
              keyPath: '/transformers/*.js/0',
            },
          ],
          '*.css': [
            {
              packageName: 'parcel-transform-css',
              resolveFrom: '.parcelrc',
              keyPath: '/transformers/*.css/0',
            },
          ],
        },
        bundler: {
          packageName: 'parcel-bundler-base',
          resolveFrom: '.parcelrc',
          keyPath: '/bundler',
        },
        runtimes: [],
        namers: [],
        optimizers: {},
        compressors: {},
        packagers: {},
        reporters: [],
        validators: {},
      };

      // $FlowFixMe
      assert.deepEqual(mergeConfigs(base, ext), merged);
    });
  });

  describe('resolveExtends', () => {
    it('should resolve a relative path', async () => {
      let resolved = await resolveExtends(
        '../.parcelrc',
        path.join(__dirname, 'fixtures', 'config', 'subfolder', '.parcelrc'),
        '/extends',
        DEFAULT_OPTIONS,
      );
      assert.equal(
        resolved,
        path.join(__dirname, 'fixtures', 'config', '.parcelrc'),
      );
    });

    it('should resolve a package name', async () => {
      let resolved = await resolveExtends(
        '@atlaspack/config-default',
        path.join(__dirname, 'fixtures', 'config', 'subfolder', '.parcelrc'),
        '/extends',
        DEFAULT_OPTIONS,
      );
      assert.equal(resolved, require.resolve('@atlaspack/config-default'));
    });
  });

  describe('parseAndProcessConfig', () => {
    it('should load and merge configs', async () => {
      let defaultConfigPath = require.resolve('@atlaspack/config-default');
      let defaultConfig = await processConfig(
        {
          ...require('@atlaspack/config-default'),
          filePath: defaultConfigPath,
        },
        DEFAULT_OPTIONS,
      );
      let configFilePath = path.join(
        __dirname,
        'fixtures',
        'config',
        '.parcelrc',
      );
      let subConfigFilePath = path.join(
        __dirname,
        'fixtures',
        'config',
        'subfolder',
        '.parcelrc',
      );
      let {config} = await parseAndProcessConfig(
        subConfigFilePath,
        DEFAULT_OPTIONS.inputFS.readFileSync(subConfigFilePath, 'utf8'),
        DEFAULT_OPTIONS,
      );

      let transformers = nullthrows(config.transformers);
      assert.deepEqual(transformers['*.js'], [
        {
          packageName: 'parcel-transformer-sub',
          resolveFrom: relative(subConfigFilePath),
          keyPath: '/transformers/*.js/0',
        },
        {
          packageName: 'parcel-transformer-base',
          resolveFrom: relative(configFilePath),
          keyPath: '/transformers/*.js/0',
        },
        '...',
      ]);
      assert(Object.keys(transformers).length > 1);
      assert.deepEqual(config.resolvers, defaultConfig.resolvers);
      assert.deepEqual(config.bundler, defaultConfig.bundler);
      assert.deepEqual(config.namers, defaultConfig.namers || []);
      assert.deepEqual(config.packagers, defaultConfig.packagers || {});
      assert.deepEqual(config.optimizers, defaultConfig.optimizers || {});
      assert.deepEqual(config.reporters, defaultConfig.reporters || []);
    });

    it('should emit a codeframe.codeHighlights when a malformed .parcelrc was found', async () => {
      let configFilePath = path.join(
        __dirname,
        'fixtures',
        'config-malformed',
        '.parcelrc',
      );
      let code = await DEFAULT_OPTIONS.inputFS.readFile(configFilePath, 'utf8');

      let pos = {
        line: 2,
        column: 14,
      };

      // $FlowFixMe[prop-missing]
      await assert.rejects(
        () => parseAndProcessConfig(configFilePath, code, DEFAULT_OPTIONS),
        {
          name: 'Error',
          diagnostics: [
            {
              message: 'Failed to parse .parcelrc',
              origin: '@atlaspack/core',
              codeFrames: [
                {
                  filePath: configFilePath,
                  language: 'json5',
                  code,
                  codeHighlights: [
                    {
                      message: "JSON5: invalid character 'b' at 2:14",
                      start: pos,
                      end: pos,
                    },
                  ],
                },
              ],
            },
          ],
        },
      );
    });

    it('should emit a codeframe when an extended parcel config file is not found', async () => {
      let configFilePath = path.join(
        __dirname,
        'fixtures',
        'config-extends-not-found',
        '.parcelrc',
      );
      let code = await DEFAULT_OPTIONS.inputFS.readFile(configFilePath, 'utf8');

      // $FlowFixMe[prop-missing]
      await assert.rejects(
        () => parseAndProcessConfig(configFilePath, code, DEFAULT_OPTIONS),
        {
          name: 'Error',
          diagnostics: [
            {
              message: 'Cannot find extended parcel config',
              origin: '@atlaspack/core',
              codeFrames: [
                {
                  filePath: configFilePath,
                  language: 'json5',
                  code,
                  codeHighlights: [
                    {
                      message:
                        '"./.parclrc-node-modules" does not exist, did you mean "./.parcelrc-node-modules"?',
                      start: {line: 2, column: 14},
                      end: {line: 2, column: 38},
                    },
                  ],
                },
              ],
            },
          ],
        },
      );
    });

    it('should emit a codeframe when an extended parcel config file is not found in JSON5', async () => {
      let configFilePath = path.join(
        __dirname,
        'fixtures',
        'config-extends-not-found',
        '.parcelrc-json5',
      );
      let code = await DEFAULT_OPTIONS.inputFS.readFile(configFilePath, 'utf8');

      // $FlowFixMe[prop-missing]
      await assert.rejects(
        () => parseAndProcessConfig(configFilePath, code, DEFAULT_OPTIONS),
        {
          name: 'Error',
          diagnostics: [
            {
              message: 'Cannot find extended parcel config',
              origin: '@atlaspack/core',
              codeFrames: [
                {
                  filePath: configFilePath,
                  language: 'json5',
                  code,
                  codeHighlights: [
                    {
                      message:
                        '"./.parclrc-node-modules" does not exist, did you mean "./.parcelrc-node-modules"?',
                      start: {line: 2, column: 12},
                      end: {line: 2, column: 36},
                    },
                  ],
                },
              ],
            },
          ],
        },
      );
    });

    it('should emit a codeframe when an extended parcel config node module is not found', async () => {
      let configFilePath = path.join(
        __dirname,
        'fixtures',
        'config-extends-not-found',
        '.parcelrc-node-modules',
      );
      let code = await DEFAULT_OPTIONS.inputFS.readFile(configFilePath, 'utf8');

      // $FlowFixMe[prop-missing]
      await assert.rejects(
        () => parseAndProcessConfig(configFilePath, code, DEFAULT_OPTIONS),
        {
          name: 'Error',
          diagnostics: [
            {
              message: 'Cannot find extended parcel config',
              origin: '@atlaspack/core',
              codeFrames: [
                {
                  filePath: configFilePath,
                  language: 'json5',
                  code,
                  codeHighlights: [
                    {
                      message:
                        'Cannot find module "@atlaspack/config-deflt", did you mean "@atlaspack/config-default"?',
                      start: {line: 2, column: 14},
                      end: {line: 2, column: 38},
                    },
                  ],
                },
              ],
            },
          ],
        },
      );
    });

    it('should emit multiple codeframes when multiple extended configs are not found', async () => {
      let configFilePath = path.join(
        __dirname,
        'fixtures',
        'config-extends-not-found',
        '.parcelrc-multiple',
      );
      let code = await DEFAULT_OPTIONS.inputFS.readFile(configFilePath, 'utf8');

      // $FlowFixMe[prop-missing]
      await assert.rejects(
        () => parseAndProcessConfig(configFilePath, code, DEFAULT_OPTIONS),
        {
          name: 'Error',
          diagnostics: [
            {
              message: 'Cannot find extended parcel config',
              origin: '@atlaspack/core',
              codeFrames: [
                {
                  filePath: configFilePath,
                  language: 'json5',
                  code,
                  codeHighlights: [
                    {
                      message:
                        'Cannot find module "@atlaspack/config-deflt", did you mean "@atlaspack/config-default"?',
                      start: {line: 2, column: 15},
                      end: {line: 2, column: 39},
                    },
                  ],
                },
              ],
            },
            {
              message: 'Cannot find extended parcel config',
              origin: '@atlaspack/core',
              codeFrames: [
                {
                  filePath: configFilePath,
                  language: 'json5',
                  code,
                  codeHighlights: [
                    {
                      message:
                        '"./.parclrc" does not exist, did you mean "./.parcelrc"?',
                      start: {line: 2, column: 42},
                      end: {line: 2, column: 53},
                    },
                  ],
                },
              ],
            },
          ],
        },
      );
    });
  });

  describe('resolve', () => {
    it('should return null if there is no .parcelrc file found', async () => {
      let resolved = await resolveAtlaspackConfig(DEFAULT_OPTIONS);
      assert.equal(resolved, null);
    });

    it('should resolve a config if a .parcelrc file is found', async () => {
      let resolved = await resolveAtlaspackConfig({
        ...DEFAULT_OPTIONS,
        projectRoot: path.join(__dirname, 'fixtures', 'config', 'subfolder'),
      });

      assert(resolved !== null);
    });
  });
});
