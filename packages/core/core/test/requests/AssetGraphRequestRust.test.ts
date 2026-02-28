import assert from 'assert';
import {getAssetGraph} from '../../src/requests/AssetGraphRequestRust';
import {createEnvironment} from '../../src/Environment';
import {fromEnvironmentId} from '../../src/EnvironmentManager';

describe('AssetGraphRequestRust -> getAssetGraph', function () {
  it('should create a valid AssetGraph', () => {
    const {assetGraph} = getAssetGraph(getSerializedGraph());

    const indexAsset = assetGraph.getNodeByContentKey('79c128d4f549c408');
    const libraryDep = assetGraph.getNodeByContentKey('cfe74f65a41af1a7');

    if (indexAsset?.type !== 'asset') return assert(false);
    if (libraryDep?.type !== 'dependency') return assert(false);

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

  describe('incremental builds with prevAssetGraph', function () {
    it('should not mutate prevAssetGraph when building a new graph', () => {
      // Build initial graph (fresh, no prev)
      const {assetGraph: prevGraph} = getAssetGraph(getSerializedGraph());
      const originalContentKeyCount = prevGraph._contentKeyToNodeId.size;
      const originalNodesLength = prevGraph.nodes.length;

      // Build incremental graph that adds new nodes
      const serialized = getSerializedGraphWithNewNode();
      getAssetGraph(serialized, prevGraph);

      // Verify prevGraph was NOT mutated.
      // With the current code, this FAILS because _contentKeyToNodeId and
      // nodes are shared by reference between the new graph and prevGraph.
      assert.equal(
        prevGraph._contentKeyToNodeId.size,
        originalContentKeyCount,
        `prevAssetGraph._contentKeyToNodeId was mutated: had ${originalContentKeyCount} keys, now has ${prevGraph._contentKeyToNodeId.size}`,
      );
      assert.equal(
        prevGraph.nodes.length,
        originalNodesLength,
        `prevAssetGraph.nodes was mutated: had ${originalNodesLength} nodes, now has ${prevGraph.nodes.length}`,
      );
    });

    it('should not throw "Graph already has content key" on retry after simulated failure', () => {
      // Build initial graph (fresh, no prev)
      const {assetGraph: prevGraph} = getAssetGraph(getSerializedGraph());

      // Simulate build N+1: getAssetGraph succeeds, but something after it
      // fails (e.g. propagateSymbols throws). The key insight: getAssetGraph
      // mutates prevGraph's Maps because they are shared by reference.
      const serialized = getSerializedGraphWithNewNode();
      getAssetGraph(serialized, prevGraph); // succeeds, but mutates prevGraph

      // Simulate build N+2: the same serialized graph is sent again by Rust
      // (Rust cached the result). prevGraph is reused because storeResult was
      // never called after the simulated failure above.
      //
      // This SHOULD succeed, but with the current code it throws
      // "Graph already has content key" because prevGraph._contentKeyToNodeId
      // already contains the new node keys from the first call.
      assert.doesNotThrow(() => {
        getAssetGraph(serialized, prevGraph);
      }, /Graph already has content key/);
    });
  });
});

function getSerializedGraph() {
  let nodes = [
    {
      type: 'root',
    },
    {
      type: 'dependency',
      value: {
        id: 'b01a088f112fa82f',
        dependency: {
          bundleBehavior: null,
          env: structuredClone(
            fromEnvironmentId(
              createEnvironment({
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
              }),
            ),
          ),
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
            env: structuredClone(
              fromEnvironmentId(
                createEnvironment({
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
                }),
              ),
            ),
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
        env: structuredClone(
          fromEnvironmentId(
            createEnvironment({
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
            }),
          ),
        ),
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
          env: structuredClone(
            fromEnvironmentId(
              createEnvironment({
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
              }),
            ),
          ),
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
        env: structuredClone(
          fromEnvironmentId(
            createEnvironment({
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
            }),
          ),
        ),
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
          env: structuredClone(
            fromEnvironmentId(
              createEnvironment({
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
              }),
            ),
          ),
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
        env: structuredClone(
          fromEnvironmentId(
            createEnvironment({
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
            }),
          ),
        ),
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
  ];

  return {
    edges: [0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6],
    nodes: nodes.map((n) => JSON.stringify(n)),
    updates: [],
  };
}

/**
 * Returns a serialized graph representing an incremental build from the base
 * graph returned by getSerializedGraph(). It simulates what Rust sends to JS
 * when a new file (utils.ts) is added and the existing index.ts is rebuilt.
 *
 * - `nodes`: two NEW nodes (a dependency + an asset for utils.ts) that Rust
 *   considers "new" (appended after starting_node_count). These will be added
 *   via addNodeByContentKey on the JS side.
 * - `updates`: one EXISTING node (the index.ts asset) that Rust considers
 *   "changed". This will be updated via getNodeByContentKey + Object.assign.
 * - `edges`: the complete edge list, including edges to the new nodes.
 *
 * After the base graph, the JS graph has nodes 0-6. The new nodes will be
 * assigned node IDs 7 and 8.
 */
function getSerializedGraphWithNewNode() {
  const esmEnv = structuredClone(
    fromEnvironmentId(
      createEnvironment({
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
      }),
    ),
  );

  // New dependency: index.ts -> ./utils (nodeId 7 in the JS graph)
  const newUtilsDep = {
    type: 'dependency',
    value: {
      id: 'dd00000000000001',
      dependency: {
        bundleBehavior: null,
        env: structuredClone(esmEnv),
        loc: {
          filePath: '/index.ts',
          start: {line: 2, column: 21},
          end: {line: 2, column: 30},
        },
        meta: {kind: 'Import'},
        pipeline: null,
        priority: 0,
        range: null,
        resolveFrom: null,
        sourceAssetId: 'cc346b5b74d3d478',
        sourcePath: '/index.ts',
        specifier: './utils',
        specifierType: 0,
        sourceAssetType: 'ts',
        symbols: [
          {
            local: '$cc346b5b74d3d478$import$dd00000000000002$2e2bcd8739ae039',
            exported: 'default',
            loc: {
              filePath: '/index.ts',
              start: {line: 2, column: 8},
              end: {line: 2, column: 15},
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
  };

  // New asset: /utils.ts (nodeId 8 in the JS graph)
  const newUtilsAsset = {
    type: 'asset',
    value: {
      id: 'dd00000000000002',
      bundleBehavior: null,
      env: structuredClone(esmEnv),
      filePath: '/utils.ts',
      type: 'js',
      meta: {
        hasCJSExports: false,
        staticExports: true,
        shouldWrap: false,
        id: 'dd00000000000003',
      },
      pipeline: null,
      query: null,
      stats: {size: 42, time: 1},
      symbols: [
        {
          local: '$dd00000000000003$export$2e2bcd8739ae039',
          exported: 'default',
          loc: {
            filePath: '/utils.ts',
            start: {line: 1, column: 1},
            end: {line: 1, column: 20},
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
  };

  // Updated existing node: index.ts asset (nodeId 4, content key 79c128d4f549c408)
  // This simulates Rust reporting index.ts as changed (rebuilt, not cached).
  const updatedIndexAsset = {
    type: 'asset',
    value: {
      id: '79c128d4f549c408',
      bundleBehavior: null,
      env: structuredClone(esmEnv),
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
      stats: {size: 180, time: 45},
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
  };

  return {
    // Edges reference JS node IDs:
    //   0=root, 1=entry_dep, 2=html_asset, 3=index_dep, 4=index_asset,
    //   5=library_dep, 6=library_asset, 7=NEW utils_dep, 8=NEW utils_asset
    edges: [0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 4, 7, 7, 8],
    // New nodes (will be added via addNodeByContentKey)
    nodes: [newUtilsDep, newUtilsAsset].map((n) => JSON.stringify(n)),
    // Updated existing nodes (will be updated via getNodeByContentKey + Object.assign)
    updates: [updatedIndexAsset].map((n) => JSON.stringify(n)),
  };
}
