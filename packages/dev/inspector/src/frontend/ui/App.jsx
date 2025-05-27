import './globals.css';
import React, {useMemo, useRef, useEffect} from 'react';
import {Outlet, Link, useParams, useSearchParams} from 'react-router';
import styles from './App.module.css';
import {useQuery} from '@tanstack/react-query';
import qs from 'qs';
import cytoscape from 'cytoscape';
import fcose from 'cytoscape-fcose';
import elk from 'cytoscape-elk';
import dagre from 'cytoscape-dagre';

cytoscape.use(fcose);
cytoscape.use(elk);
cytoscape.use(dagre);

function formatBytes(bytes) {
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  if (bytes === 0) return '0 B';
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return (bytes / Math.pow(1024, i)).toFixed(2) + ' ' + sizes[i];
}

export function Stats() {
  const {data, isLoading, error} = useQuery({
    queryKey: ['/api/stats'],
  });
  const {
    data: assetGraph,
    isLoading: isLoadingAssetGraph,
    error: errorAssetGraph,
  } = useQuery({
    queryKey: ['/api/asset-graph'],
  });

  const graphContainerRef = useRef(null);
  useEffect(() => {
    if (graphContainerRef.current) {
      const instance = cytoscape({
        container: graphContainerRef.current,
        // userZoomingEnabled: true,
        // userPanningEnabled: false,
        boxSelectionEnabled: false,
        // autolock: true,
        autounselectify: true,
        style: [
          {
            selector: 'node',
            style: {
              'background-color': '#000',
              color: '#fff',
            },
          },
          {
            selector: 'node[label]',
            style: {
              color: 'red',
              label: 'data(label)',
            },
          },
          {
            selector: 'edge',
            style: {
              // 'curve-style': 'taxi',
              // 'taxi-direction': 'rightward',
              'target-arrow-shape': 'triangle',
              'arrow-scale': 0.66,
            },
          },
        ],
        elements: [
          ...assetGraph.nodes.map((node) => ({
            data: {id: node.id, label: node.id === '@@root' ? 'Root node' : ''},
            grabbable: false,
          })),
          ...assetGraph.nodes.flatMap((node) =>
            node.edges.map((edge) => ({
              data: {
                id: `${node.id}-${edge}`,
                source: node.id,
                target: edge,
              },
            })),
          ),
        ],
        layout: {
          name: 'dagre',
          // nodeSep: 200,
          // elk: {
          //   algorithm: 'mrtree',
          //   direction: 'DOWN',
          // },
        },
      });

      return () => {
        instance.destroy();
      };
    }
  }, [assetGraph]);

  if (isLoading || isLoadingAssetGraph) {
    return <div>Loading...</div>;
  }

  if (error || errorAssetGraph) {
    return <div>Error: {error.message || errorAssetGraph.message}</div>;
  }

  return (
    <div>
      <h2>stats</h2>
      <h3>asset graph</h3>
      <div ref={graphContainerRef} style={{width: '100%', height: '100vh'}} />
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
    </div>
  );
}

export function CacheKeyList({cacheKeys, sortBy, setSortBy, isLoading, error}) {
  const [searchParams] = useSearchParams();

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (error) {
    return <div>Error: {error.message}</div>;
  }

  return (
    <>
      <label>Sort by</label>
      <select value={sortBy} onChange={(e) => setSortBy(e.target.value)}>
        <option value="order">order</option>
        <option value="size">size</option>
      </select>

      <ul>
        {cacheKeys.keys.map((key) => (
          <li key={key}>
            <Link
              to={`/app/cache/${encodeURIComponent(
                key,
              )}?${searchParams.toString()}`}
            >
              {key}
            </Link>
          </li>
        ))}
      </ul>
    </>
  );
}

export function CacheValue() {
  const key = useParams().key;

  const {
    data: cacheValue,
    isLoading,
    error,
  } = useQuery({
    queryKey: [`/api/cache-value/${encodeURIComponent(key)}`],
  });

  const content = () => {
    if (isLoading) {
      return <div>Loading...</div>;
    }

    if (error) {
      return <div>Error: {error.message}</div>;
    }

    return (
      <>
        <p>size: {formatBytes(cacheValue.size)}</p>

        <pre>
          <code>{cacheValue.value}</code>
        </pre>
      </>
    );
  };

  return (
    <div>
      <h2>cache value</h2>
      <h3>key: {key}</h3>

      {content()}
    </div>
  );
}

export default function App() {
  // sort by in querystring URL
  const [searchParams, setSearchParams] = useSearchParams();
  const sortBy = searchParams.get('sortBy') || 'order';
  const setSortBy = (value) => {
    searchParams.set('sortBy', value);
    setSearchParams(searchParams);
  };

  const {
    data: cacheKeys,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['/api/cache-keys/?' + qs.stringify({sortBy})],
  });

  return (
    <div className={styles.app}>
      <div className={styles.sidebar}>
        <h2>cache keys</h2>
        <Link to="/">stats</Link>

        <hr />

        <CacheKeyList
          isLoading={isLoading}
          error={error}
          cacheKeys={cacheKeys}
          sortBy={sortBy}
          setSortBy={setSortBy}
        />
      </div>

      <div className={styles.content}>
        <div className={styles.contentInner}>
          <Outlet />
        </div>
      </div>
    </div>
  );
}
