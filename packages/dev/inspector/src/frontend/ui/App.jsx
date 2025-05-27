import './globals.css';
import React from 'react';
import {Outlet, Link, useParams, useSearchParams} from 'react-router';
import styles from './App.module.css';
import {useQuery} from '@tanstack/react-query';
import qs from 'qs';

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

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (error) {
    return <div>Error: {error.message}</div>;
  }

  return (
    <div>
      <h2>stats</h2>
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
