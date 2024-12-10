import assert from 'assert';
import {getAssetGraph} from '../../src/requests/AssetGraphRequestRust';

describe('AssetGraphRequestRust -> getAssetGraph', function () {
  it('should create a valid AssetGraph', () => {
    const {assetGraph} = getAssetGraph(getSerializedGraph());

    const indexAsset = assetGraph.getNodeByContentKey('79c128d4f549c408');
    const libraryDep = assetGraph.getNodeByContentKey('cfe74f65a41af1a7');

    assert(indexAsset?.type === 'asset');
    assert(libraryDep?.type === 'dependency');

    assert.equal(indexAsset.value.filePath, '/index.ts');
    assert.equal(libraryDep.value.specifier, './library');
    assert.deepEqual(
      indexAsset.value.dependencies.get('cfe74f65a41af1a7')?.specifier,
      './library',
    );

    const indexAssetNodeId =
      assetGraph.getNodeIdByContentKey('79c128d4f549c408');
    const libraryDepNodeId =
      assetGraph.getNodeIdByContentKey('cfe74f65a41af1a7');
    assert.deepEqual(assetGraph.getNodeIdsConnectedFrom(indexAssetNodeId), [
      libraryDepNodeId,
    ]);
  });
});

function getSerializedGraph() {
  return {
    edges: [0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6],
    nodes: [
      {
        type: 'root',
      },
      {
        type: 'dependency',
        value: {
          id: 'b01a088f112fa82f',
          dependency: {
            bundleBehavior: null,
            env: {
              context: 'browser',
              engines: {
                browsers: [
                  'last 1 Chrome version',
                  'last 1 Safari version',
                  'last 1 Firefox version',
                  'last 1 Edge version',
                ],
              },
              includeNodeModules: true,
              isLibrary: false,
              loc: null,
              outputFormat: 'global',
              shouldScopeHoist: true,
              shouldOptimize: false,
              sourceMap: {},
              sourceType: 'module',
            },
            loc: null,
            meta: {},
            pipeline: null,
            priority: 0,
            range: null,
            resolveFrom: null,
            sourceAssetId: null,
            sourcePath: null,
            specifier: '/index.html',
            specifierType: 2,
            symbols: null,
            target: {
              distDir: '/dist',
              distEntry: null,
              env: {
                context: 'browser',
                engines: {
                  browsers: [
                    'last 1 Chrome version',
                    'last 1 Safari version',
                    'last 1 Firefox version',
                    'last 1 Edge version',
                  ],
                },
                includeNodeModules: true,
                isLibrary: false,
                loc: null,
                outputFormat: 'global',
                shouldScopeHoist: true,
                shouldOptimize: false,
                sourceMap: {},
                sourceType: 'module',
              },
              loc: null,
              name: 'default',
              publicUrl: '/',
            },
            isEntry: true,
            isOptional: false,
            needsStableName: true,
            shouldWrap: false,
            isEsm: false,
            placeholder: null,
          },
        },
        has_deferred: false,
      },
      {
        type: 'asset',
        value: {
          id: 'e2056518260d7dc7',
          bundleBehavior: 1,
          env: {
            context: 'browser',
            engines: {
              browsers: [
                'last 1 Chrome version',
                'last 1 Safari version',
                'last 1 Firefox version',
                'last 1 Edge version',
              ],
            },
            includeNodeModules: true,
            isLibrary: false,
            loc: null,
            outputFormat: 'global',
            shouldScopeHoist: true,
            shouldOptimize: false,
            sourceMap: {},
            sourceType: 'module',
          },
          filePath: '/index.html',
          type: 'html',
          meta: {},
          pipeline: null,
          query: null,
          stats: {
            size: 93,
            time: 2,
          },
          symbols: null,
          sideEffects: true,
          isBundleSplittable: true,
          isSource: true,
          hasCjsExports: false,
          staticExports: false,
          shouldWrap: false,
          hasNodeReplacements: false,
          isConstantModule: false,
          conditions: [],
          configPath: null,
          configKeyPath: null,
        },
      },
      {
        type: 'dependency',
        value: {
          id: 'aece0f57a78d1ef8',
          dependency: {
            bundleBehavior: null,
            env: {
              context: 'browser',
              engines: {
                browsers: [
                  'last 1 Chrome version',
                  'last 1 Safari version',
                  'last 1 Firefox version',
                  'last 1 Edge version',
                ],
              },
              includeNodeModules: true,
              isLibrary: false,
              loc: null,
              outputFormat: 'esmodule',
              shouldScopeHoist: true,
              shouldOptimize: false,
              sourceMap: {},
              sourceType: 'module',
            },
            loc: null,
            meta: {},
            pipeline: null,
            priority: 1,
            range: null,
            resolveFrom: null,
            sourceAssetId: 'e2056518260d7dc7',
            sourcePath: '/index.html',
            specifier: './index.ts',
            specifierType: 2,
            sourceAssetType: 'html',
            symbols: null,
            target: null,
            isEntry: false,
            isOptional: false,
            needsStableName: false,
            shouldWrap: false,
            isEsm: true,
            placeholder: null,
          },
        },
        has_deferred: false,
      },
      {
        type: 'asset',
        value: {
          id: '79c128d4f549c408',
          bundleBehavior: null,
          env: {
            context: 'browser',
            engines: {
              browsers: [
                'last 1 Chrome version',
                'last 1 Safari version',
                'last 1 Firefox version',
                'last 1 Edge version',
              ],
            },
            includeNodeModules: true,
            isLibrary: false,
            loc: null,
            outputFormat: 'esmodule',
            shouldScopeHoist: true,
            shouldOptimize: false,
            sourceMap: {},
            sourceType: 'module',
          },
          filePath: '/index.ts',
          type: 'js',
          meta: {
            hasCJSExports: false,
            staticExports: true,
            shouldWrap: false,
            id: 'cc346b5b74d3d478',
          },
          pipeline: null,
          query: null,
          stats: {
            size: 121,
            time: 39,
          },
          symbols: [],
          sideEffects: true,
          isBundleSplittable: true,
          isSource: true,
          hasCjsExports: false,
          staticExports: true,
          shouldWrap: false,
          hasNodeReplacements: false,
          isConstantModule: false,
          conditions: [],
          configPath: null,
          configKeyPath: null,
        },
      },
      {
        type: 'dependency',
        value: {
          id: 'cfe74f65a41af1a7',
          dependency: {
            bundleBehavior: null,
            env: {
              context: 'browser',
              engines: {
                browsers: [
                  'last 1 Chrome version',
                  'last 1 Safari version',
                  'last 1 Firefox version',
                  'last 1 Edge version',
                ],
              },
              includeNodeModules: true,
              isLibrary: false,
              loc: null,
              outputFormat: 'esmodule',
              shouldScopeHoist: true,
              shouldOptimize: false,
              sourceMap: {},
              sourceType: 'module',
            },
            loc: {
              filePath: '/index.ts',
              start: {
                line: 1,
                column: 21,
              },
              end: {
                line: 1,
                column: 32,
              },
            },
            meta: {
              kind: 'Import',
            },
            pipeline: null,
            priority: 0,
            range: null,
            resolveFrom: null,
            sourceAssetId: 'cc346b5b74d3d478',
            sourcePath: '/index.ts',
            specifier: './library',
            specifierType: 0,
            sourceAssetType: 'ts',
            symbols: [
              {
                local:
                  '$cc346b5b74d3d478$import$b83d1a328c413a1a$2e2bcd8739ae039',
                exported: 'default',
                loc: {
                  filePath: '/index.ts',
                  start: {
                    line: 1,
                    column: 8,
                  },
                  end: {
                    line: 1,
                    column: 15,
                  },
                },
                isWeak: false,
                isEsmExport: false,
                selfReferenced: false,
              },
            ],
            target: null,
            isEntry: false,
            isOptional: false,
            needsStableName: false,
            shouldWrap: false,
            isEsm: true,
            placeholder: null,
          },
        },
        has_deferred: false,
      },
      {
        type: 'asset',
        value: {
          id: '13fc7969eb974fe3',
          bundleBehavior: null,
          env: {
            context: 'browser',
            engines: {
              browsers: [
                'last 1 Chrome version',
                'last 1 Safari version',
                'last 1 Firefox version',
                'last 1 Edge version',
              ],
            },
            includeNodeModules: true,
            isLibrary: false,
            loc: null,
            outputFormat: 'esmodule',
            shouldScopeHoist: true,
            shouldOptimize: false,
            sourceMap: {},
            sourceType: 'module',
          },
          filePath: '/library.ts',
          type: 'js',
          meta: {
            hasCJSExports: false,
            staticExports: true,
            shouldWrap: false,
            id: '579fff58dabc69a5',
          },
          pipeline: null,
          query: null,
          stats: {
            size: 58,
            time: 1,
          },
          symbols: [
            {
              local: '$579fff58dabc69a5$export$2e2bcd8739ae039',
              exported: 'default',
              loc: {
                filePath: '/library.ts',
                start: {
                  line: 1,
                  column: 1,
                },
                end: {
                  line: 1,
                  column: 26,
                },
              },
              isWeak: false,
              isEsmExport: true,
              selfReferenced: false,
            },
          ],
          sideEffects: true,
          isBundleSplittable: true,
          isSource: true,
          hasCjsExports: false,
          staticExports: true,
          shouldWrap: false,
          hasNodeReplacements: false,
          isConstantModule: false,
          conditions: [],
          configPath: null,
          configKeyPath: null,
        },
      },
    ],
  };
}
