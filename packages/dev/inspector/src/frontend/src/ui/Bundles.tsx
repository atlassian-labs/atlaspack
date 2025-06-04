import {useSearchParams} from 'react-router';
import {useQuery} from '@tanstack/react-query';
import qs from 'qs';
import {Graph} from './Graph';
import {GraphRenderer} from './GraphRenderer';

export function Bundles() {
  const [searchParams] = useSearchParams();
  const rootNodeId = searchParams.get('rootNodeId');

  const {
    data: bundleGraph,
    isLoading: isLoadingBundleGraph,
    error: errorBundleGraph,
  } = useQuery<Graph>({
    queryKey: [`/api/bundle-graph?${qs.stringify({rootNodeId})}`],
  });

  if (isLoadingBundleGraph) {
    return <div>Loading...</div>;
  }

  if (errorBundleGraph) {
    return <div>Error: {errorBundleGraph.message}</div>;
  }

  if (!bundleGraph) {
    throw new Error('No bundle graph');
  }

  return (
    <div
      style={{
        flex: 1,
        minHeight: '100%',
        display: 'flex',
        flexDirection: 'column',
      }}
    >
      <h2>bundle graph</h2>

      <GraphRenderer graph={bundleGraph} graphType="bundle-graph" />
    </div>
  );
}
