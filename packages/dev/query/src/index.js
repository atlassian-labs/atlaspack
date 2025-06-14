// @flow strict-local
/* eslint-disable no-console, monorepo/no-internal-import */
import type {ContentKey, NodeId} from '@atlaspack/graph';
import type {PackagedBundleInfo} from '@atlaspack/core/src/types';

import v8 from 'v8';
import nullthrows from 'nullthrows';
import invariant from 'assert';

const {
  AssetGraph,
  BundleGraph: {default: BundleGraph},
  RequestTracker: {
    default: RequestTracker,
    readAndDeserializeRequestGraph,
    requestGraphEdgeTypes,
  },
  LMDBLiteCache,
} = require('./deep-imports.js');

export async function loadGraphs(cacheDir: string): Promise<{|
  assetGraph: ?AssetGraph,
  bundleGraph: ?BundleGraph,
  requestTracker: ?RequestTracker,
  bundleInfo: ?Map<ContentKey, PackagedBundleInfo>,
  cacheInfo: ?Map<string, Array<string | number>>,
|}> {
  let cacheInfo: Map<string, Array<string | number>> = new Map();
  const cache = new LMDBLiteCache(cacheDir);

  let requestGraphBlob;
  let requestGraphKey;
  let bundleGraphBlob;
  let assetGraphBlob;
  for (let key of cache.keys()) {
    if (key.startsWith('Asset/')) {
      continue;
    } else if (key.startsWith('PackagerRunner/')) {
      continue;
    }

    if (key.startsWith('RequestTracker/') && key.endsWith('/RequestGraph')) {
      requestGraphBlob = key;
      requestGraphKey = key.split('/').slice(0, -1).join('/');
    }
    if (key.startsWith('BundleGraph/')) {
      bundleGraphBlob = key;
    }
    if (key.startsWith('AssetGraph/')) {
      assetGraphBlob = key;
    }
  }

  console.log({requestGraphBlob, bundleGraphBlob, assetGraphBlob});

  // Get requestTracker
  let requestTracker;
  if (requestGraphBlob != null && requestGraphKey != null) {
    try {
      let date = Date.now();

      const buffer = await cache.getBlob(requestGraphBlob);
      const deserializer = new v8.Deserializer(buffer);
      console.log(
        'Wire format version stored',
        deserializer.getWireFormatVersion(),
      );

      let {requestGraph, bufferLength} = await readAndDeserializeRequestGraph(
        cache,
        requestGraphBlob,
        requestGraphKey,
      );

      requestTracker = new RequestTracker({
        graph: requestGraph,
        // $FlowFixMe
        farm: null,
        // $FlowFixMe
        options: null,
      });
      let timeToDeserialize = Date.now() - date;
      cacheInfo.set('RequestGraph', [bufferLength]);
      cacheInfo.get('RequestGraph')?.push(timeToDeserialize);
    } catch (e) {
      console.error('Error loading Request Graph\n', e);
    }
  }

  // Get bundleGraph
  let bundleGraph;
  if (bundleGraphBlob != null) {
    try {
      let file = await cache.getBlob(bundleGraphBlob);

      let timeToDeserialize = Date.now();
      let obj = v8.deserialize(file);
      invariant(obj.bundleGraph != null);
      bundleGraph = BundleGraph.deserialize(obj.bundleGraph.value);
      timeToDeserialize = Date.now() - timeToDeserialize;

      cacheInfo.set('BundleGraph', [Buffer.byteLength(file)]);
      cacheInfo.get('BundleGraph')?.push(timeToDeserialize);
    } catch (e) {
      console.error('Error loading Bundle Graph\n', e);
    }
  }

  // Get assetGraph
  let assetGraph;
  if (assetGraphBlob != null) {
    try {
      // TODO: this should be reviewed when `cachePerformanceImprovements` flag is removed, as we'll be writing files to LMDB cache instead of large blobs
      let file = await cache.getBlob(assetGraphBlob);

      let timeToDeserialize = Date.now();
      let obj = v8.deserialize(file);
      invariant(obj.assetGraph != null);
      assetGraph = AssetGraph.deserialize(obj.assetGraph.value);
      timeToDeserialize = Date.now() - timeToDeserialize;

      cacheInfo.set('AssetGraph', [Buffer.byteLength(file)]);
      cacheInfo.get('AssetGraph')?.push(timeToDeserialize);
    } catch (e) {
      console.error('Error loading Asset Graph\n', e);
    }
  }

  function getSubRequests(id: NodeId) {
    return requestTracker.graph
      .getNodeIdsConnectedFrom(id, requestGraphEdgeTypes.subrequest)
      .map((n) => nullthrows(requestTracker.graph.getNode(n)));
  }

  // Load graphs by finding the main subrequests and loading their results
  let bundleInfo;
  try {
    invariant(requestTracker);
    let buildRequestId = requestTracker.graph.getNodeIdByContentKey(
      'atlaspack_build_request',
    );
    let buildRequestNode = nullthrows(
      requestTracker.graph.getNode(buildRequestId),
    );
    invariant(
      buildRequestNode.type === 1 && buildRequestNode.requestType === 1,
    );
    let buildRequestSubRequests = getSubRequests(buildRequestId);

    let writeBundlesRequest = buildRequestSubRequests.find(
      (n) => n.type === 1 && n.requestType === 11,
    );
    if (writeBundlesRequest != null) {
      invariant(writeBundlesRequest.type === 1);
      // $FlowFixMe[incompatible-cast]
      bundleInfo = (nullthrows(writeBundlesRequest.result): Map<
        ContentKey,
        PackagedBundleInfo,
      >);
    }
  } catch (e) {
    console.error('Error loading bundleInfo\n', e);
  }

  return {assetGraph, bundleGraph, requestTracker, bundleInfo, cacheInfo};
}
