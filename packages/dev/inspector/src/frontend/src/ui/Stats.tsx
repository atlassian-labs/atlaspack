import {useSearchParams} from 'react-router';
import {useQuery} from '@tanstack/react-query';
import qs from 'qs';
import {formatBytes} from './formatBytes';
import {Graph} from './Graph';
import {GraphRenderer} from './GraphRenderer';

export function Stats() {
  const [searchParams] = useSearchParams();
  const rootNodeId = searchParams.get('rootNodeId');

  const {data, isLoading, error} = useQuery<{
    size: number;
    keySize: number;
    count: number;
    assetContentCount: number;
    assetContentSize: number;
    assetMapCount: number;
    assetMapSize: number;
  }>({
    queryKey: ['/api/stats'],
  });
  const {
    data: assetGraph,
    isLoading: isLoadingAssetGraph,
    error: errorAssetGraph,
  } = useQuery<Graph>({
    queryKey: [`/api/asset-graph?${qs.stringify({rootNodeId})}`],
  });

  if (!data || isLoading || isLoadingAssetGraph) {
    return <div>Loading...</div>;
  }

  if (error || errorAssetGraph) {
    return <div>Error: {error?.message || errorAssetGraph?.message}</div>;
  }

  if (!assetGraph) {
    throw new Error('No asset graph');
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
      <h2>stats</h2>
      <h3>asset graph</h3>

      <table>
        <tbody>
          <tr>
            <td>size</td>
            <td>{formatBytes(data.size)}</td>
          </tr>
          <tr>
            <td>key size</td>
            <td>{formatBytes(data.keySize)}</td>
          </tr>
          <tr>
            <td>count</td>
            <td>{data.count}</td>
          </tr>

          <tr>
            <td>asset content count</td>
            <td>{data.assetContentCount}</td>
          </tr>
          <tr>
            <td>asset content size</td>
            <td>{formatBytes(data.assetContentSize)}</td>
          </tr>

          <tr>
            <td>asset map count</td>
            <td>{data.assetMapCount}</td>
          </tr>
          <tr>
            <td>asset map size</td>
            <td>{formatBytes(data.assetMapSize)}</td>
          </tr>
        </tbody>
      </table>

      <GraphRenderer graph={assetGraph} graphType="asset-graph" />
    </div>
  );
}
