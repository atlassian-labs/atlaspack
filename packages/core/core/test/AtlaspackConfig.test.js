// @flow strict-local

import {describe, it} from 'node:test';
import assert from 'assert';
import path from 'path';

import {AtlaspackConfig} from '../src/AtlaspackConfig';
import {toProjectPath} from '../src/projectPath';
import {parseAndProcessConfig} from '../src/requests/AtlaspackConfigRequest';

import {DEFAULT_OPTIONS} from './test-utils';

const ATLASPACKRC_PATH = toProjectPath('/', '/.parcelrc');

describe('AtlaspackConfig', () => {
  describe('matchGlobMap', () => {
    let config = new AtlaspackConfig(
      {
        filePath: ATLASPACKRC_PATH,
        bundler: undefined,
        packagers: {
          '*.css': {
            packageName: 'parcel-packager-css',
            resolveFrom: ATLASPACKRC_PATH,
            keyPath: '/packagers/*.css',
          },
          '*.js': {
            packageName: 'parcel-packager-js',
            resolveFrom: ATLASPACKRC_PATH,
            keyPath: '/packagers/*.js',
          },
        },
      },
      DEFAULT_OPTIONS,
    );

    it('should return null array if no glob matches', () => {
      let result = config.matchGlobMap(
        toProjectPath('/', '/foo.wasm'),
        config.packagers,
      );
      assert.deepEqual(result, null);
    });

    it('should return a matching pipeline', () => {
      let result = config.matchGlobMap(
        toProjectPath('/', '/foo.js'),
        config.packagers,
      );
      assert.deepEqual(result, {
        packageName: 'parcel-packager-js',
        resolveFrom: ATLASPACKRC_PATH,
        keyPath: '/packagers/*.js',
      });
    });
  });

  describe('matchGlobMapPipelines', () => {
    let config = new AtlaspackConfig(
      {
        filePath: ATLASPACKRC_PATH,
        bundler: undefined,
        transformers: {
          '*.jsx': [
            {
              packageName: 'parcel-transform-jsx',
              resolveFrom: ATLASPACKRC_PATH,
              keyPath: '/transformers/*.jsx/0',
            },
            '...',
          ],
          '*.{js,jsx}': [
            {
              packageName: 'parcel-transform-js',
              resolveFrom: ATLASPACKRC_PATH,
              keyPath: '/transformers/*.{js,jsx}/0',
            },
          ],
        },
      },
      DEFAULT_OPTIONS,
    );

    it('should return an empty array if no pipeline matches', () => {
      let pipeline = config.matchGlobMapPipelines(
        toProjectPath('/', '/foo.css'),
        config.transformers,
      );
      assert.deepEqual(pipeline, []);
    });

    it('should return a matching pipeline', () => {
      let pipeline = config.matchGlobMapPipelines(
        toProjectPath('/', '/foo.js'),
        config.transformers,
      );
      assert.deepEqual(pipeline, [
        {
          packageName: 'parcel-transform-js',
          resolveFrom: ATLASPACKRC_PATH,
          keyPath: '/transformers/*.{js,jsx}/0',
        },
      ]);
    });

    it('should merge pipelines with spread elements', () => {
      let pipeline = config.matchGlobMapPipelines(
        toProjectPath('/', '/foo.jsx'),
        config.transformers,
      );
      assert.deepEqual(pipeline, [
        {
          packageName: 'parcel-transform-jsx',
          resolveFrom: ATLASPACKRC_PATH,
          keyPath: '/transformers/*.jsx/0',
        },
        {
          packageName: 'parcel-transform-js',
          resolveFrom: ATLASPACKRC_PATH,
          keyPath: '/transformers/*.{js,jsx}/0',
        },
      ]);
    });
  });

  describe('loadPlugin', () => {
    it('should error with a codeframe if a plugin is not resolved', async () => {
      let configFilePath = path.join(
        __dirname,
        'fixtures',
        'config-plugin-not-found',
        '.parcelrc',
      );
      let code = await DEFAULT_OPTIONS.inputFS.readFile(configFilePath, 'utf8');
      let {config} = await parseAndProcessConfig(
        configFilePath,
        code,
        DEFAULT_OPTIONS,
      );
      let atlaspackConfig = new AtlaspackConfig(config, DEFAULT_OPTIONS);

      // $FlowFixMe
      await assert.rejects(() => atlaspackConfig.getTransformers('test.js'), {
        name: 'Error',
        diagnostics: [
          {
            message: 'Cannot find Atlaspack plugin "@atlaspack/transformer-jj"',
            origin: '@atlaspack/core',
            codeFrames: [
              {
                filePath: configFilePath,
                language: 'json5',
                code,
                codeHighlights: [
                  {
                    start: {line: 4, column: 14},
                    end: {line: 4, column: 40},
                    message: `Cannot find module "@atlaspack/transformer-jj", did you mean "@atlaspack/transformer-js"?`,
                  },
                ],
              },
            ],
          },
        ],
      });
    });

    it('should error when using a reserved pipeline name "node:*"', async () => {
      let configFilePath = path.join(
        __dirname,
        'fixtures',
        'config-node-pipeline',
        '.parcelrc',
      );
      let code = await DEFAULT_OPTIONS.inputFS.readFile(configFilePath, 'utf8');

      // $FlowFixMe
      await assert.rejects(
        () => parseAndProcessConfig(configFilePath, code, DEFAULT_OPTIONS),
        {
          name: 'Error',
          diagnostics: [
            {
              message: "Named pipeline 'node:' is reserved.",
              origin: '@atlaspack/core',
              codeFrames: [
                {
                  filePath: configFilePath,
                  language: 'json5',
                  code,
                  codeHighlights: [
                    {
                      message: undefined,
                      start: {
                        line: 4,
                        column: 5,
                      },
                      end: {
                        line: 4,
                        column: 15,
                      },
                    },
                  ],
                },
              ],
              documentationURL:
                'https://parceljs.org/features/dependency-resolution/#url-schemes',
            },
          ],
        },
      );
    });

    it('should support loading local plugins', async () => {
      let projectRoot = path.join(__dirname, 'fixtures', 'plugins');
      let configFilePath = toProjectPath(
        projectRoot,
        path.join(__dirname, 'fixtures', 'plugins', '.parcelrc'),
      );
      let config = new AtlaspackConfig(
        {
          filePath: configFilePath,
          bundler: undefined,
          transformers: {
            '*.js': [
              {
                packageName: './local-plugin',
                resolveFrom: configFilePath,
                keyPath: '/transformers/*.js/0',
              },
            ],
          },
        },
        {...DEFAULT_OPTIONS, projectRoot},
      );

      let [{plugin}] = await config.getTransformers(
        toProjectPath('/', '/foo.js'),
      );
      assert(plugin);
      assert.equal(typeof plugin.transform, 'function');
    });

    it('should error on local plugins inside config packages', async () => {
      let configFilePath = path.join(
        __dirname,
        'fixtures',
        'local-plugin-config-pkg',
        '.parcelrc',
      );
      let code = await DEFAULT_OPTIONS.inputFS.readFile(configFilePath, 'utf8');
      let {config} = await parseAndProcessConfig(
        configFilePath,
        code,
        DEFAULT_OPTIONS,
      );
      let atlaspackConfig = new AtlaspackConfig(config, DEFAULT_OPTIONS);
      let extendedConfigPath = path.join(
        __dirname,
        'fixtures',
        'local-plugin-config-pkg',
        'node_modules',
        'parcel-config-local',
        'index.json',
      );

      // $FlowFixMe
      await assert.rejects(() => atlaspackConfig.getTransformers('test.js'), {
        name: 'Error',
        diagnostics: [
          {
            message:
              'Local plugins are not supported in Atlaspack config packages. Please publish "./local-plugin" as a separate npm package.',
            origin: '@atlaspack/core',
            codeFrames: [
              {
                filePath: extendedConfigPath,
                language: 'json5',
                code: await DEFAULT_OPTIONS.inputFS.readFile(
                  extendedConfigPath,
                  'utf8',
                ),
                codeHighlights: [
                  {
                    start: {line: 5, column: 7},
                    end: {line: 5, column: 22},
                    message: undefined,
                  },
                ],
              },
            ],
          },
        ],
      });
    });
  });
});
