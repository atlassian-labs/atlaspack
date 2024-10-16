// // @flow strict-local
//
// import MutableBundleGraph from '@atlaspack/core/src/public/MutableBundleGraph';
// import InternalBundleGraph from '@atlaspack/core/src/BundleGraph';
// import type {AtlaspackOptions, Dependency} from '@atlaspack/core/src/types';
// import resolveOptions from '@atlaspack/core/src/resolveOptions';
// import AssetGraph from '@atlaspack/core/src/AssetGraph';
// import {getParcelOptions} from '@atlaspack/test-utils/src/utils';
// import {createIdealGraph, type IdealGraph} from '../src/DefaultBundler';
// import sinon from 'sinon';
// import assert from 'assert';
// import {toProjectPath} from '@atlaspack/core/src/projectPath';
// import {createEnvironment} from '@atlaspack/core/src/Environment';
// import nullthrows from 'nullthrows';
// import type {Dependency as IDependency} from '@atlaspack/types';
// import type {PublicAsset as CoreAsset} from '@atlaspack/core/src/types';
// import type {PublicAsset as IAsset} from '@atlaspack/types';
// import {assetFromValue} from '@atlaspack/core/src/public/PublicAsset';
// import {MutableDependencySymbols} from '@atlaspack/core/src/public/Symbols';
// import {createDependency} from '@atlaspack/core/src/Dependency';
// import type {DependencyOptions} from '@atlaspack/types';
//
// describe.only('DefaultBundler', () => {
//   describe('createIdealGraph', () => {
//     describe('on an asset graph with a single asset', () => {
//       it('produces a single output bundle containing the asset', async () => {
//         const atlaspackOptions: AtlaspackOptions = await resolveOptions(
//           getParcelOptions('entry.js'),
//         );
//         const assetGraph = new AssetGraph();
//         const coreAsset: CoreAsset = {
//           id: '123456789',
//           committed: true,
//           filePath: toProjectPath('', 'entry.js'),
//           type: 'js',
//           dependencies: new Map(),
//           isBundleSplittable: true,
//           isSource: true,
//           env: createEnvironment({}),
//           meta: {},
//           stats: {
//             size: 0,
//             time: 0,
//           },
//           sideEffects: false,
//           astGenerator: null,
//           mapKey: null,
//           pipeline: null,
//           outputHash: null,
//           plugin: null,
//           query: null,
//           symbols: null,
//           uniqueKey: null,
//           astKey: null,
//           bundleBehavior: null,
//           contentKey: null,
//         };
//         const asset: IAsset = assetFromValue(coreAsset, atlaspackOptions);
//         const nodeId = assetGraph.addNode({
//           id: '123456789',
//           type: 'asset',
//           usedSymbols: new Set(),
//           usedSymbolsDownDirty: false,
//           usedSymbolsUpDirty: false,
//           value: coreAsset,
//         });
//         const assetGroupNodeId = assetGraph.addNode({
//           id: '987654321',
//           type: 'asset_group',
//           usedSymbolsDownDirty: false,
//           value: {
//             filePath: toProjectPath('', 'entry.js'),
//             canDefer: false,
//             code: undefined,
//             env: createEnvironment({}),
//             isSingleChangeRebuild: false,
//             isSource: true,
//             isURL: false,
//             name: 'entry.js',
//             pipeline: null,
//             query: null,
//             sideEffects: false,
//           },
//         });
//         assetGraph.addEdge(assetGroupNodeId, nodeId);
//         assetGraph.replaceNodeIdsConnectedTo(
//           nullthrows(assetGraph.rootNodeId),
//           [assetGroupNodeId],
//         );
//
//         const internalBundleGraph = InternalBundleGraph.fromAssetGraph(
//           assetGraph,
//           false,
//         );
//         const mutableBundleGraph = new MutableBundleGraph(
//           internalBundleGraph,
//           atlaspackOptions,
//         );
//         const config = {
//           maxParallelRequests: 10,
//           minBundleSize: 10,
//           minBundles: 10,
//           projectRoot: '',
//           disableSharedBundles: false,
//           manualSharedBundles: [],
//         };
//         const entries: Map<IAsset, IDependency> = new Map();
//         const dependencyOptions: DependencyOptions = {
//           env: {},
//           isOptional: false,
//           meta: {},
//           needsStableName: false,
//           packageConditions: undefined,
//           priority: 'sync',
//           specifier: 'entry.js',
//           specifierType: 'url',
//           symbols: new Map(),
//         };
//         const dependency: Dependency = createDependency('', dependencyOptions);
//         entries.set(asset, dependency);
//         const logger = {
//           error: sinon.stub(),
//           info: sinon.stub(),
//           warn: sinon.stub(),
//           debug: sinon.stub(),
//           verbose: sinon.stub(),
//           log: sinon.stub(),
//         };
//
//         const idealGraph: IdealGraph = createIdealGraph(
//           mutableBundleGraph,
//           config,
//           entries,
//           logger,
//         );
//
//         assert.equal(idealGraph.bundleGraph.nodes.length, 1);
//         const bundle = idealGraph.bundleGraph.nodes[0];
//         if (bundle?.type === 'bundle') {
//         } else {
//           throw new Error(
//             'expected bundle graph to contain bundle but it contained ' +
//               String(bundle),
//           );
//         }
//       });
//     });
//   });
// });
