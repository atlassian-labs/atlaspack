import {Link, useSearchParams} from 'react-router';
import {useQuery} from '@tanstack/react-query';
import qs from 'qs';
import styles from './App.module.css';
import {CacheKeyList} from './CacheKeyList';

export function Sidebar() {
  // sort by in querystring URL
  const [searchParams, setSearchParams] = useSearchParams();
  const sortBy = searchParams.get('sortBy') || 'order';
  const setSortBy = (value: string) => {
    searchParams.set('sortBy', value);
    setSearchParams(searchParams);
  };

  const {
    data: cacheKeys,
    isLoading,
    error,
  } = useQuery<{
    keys: string[];
  }>({
    queryKey: ['/api/cache-keys/?' + qs.stringify({sortBy})],
  });

  return (
    <div className={styles.sidebar}>
      <h1>atlaspack</h1>
      <ul>
        <li className={styles.sidebarItem}>
          <Link to="/">stats</Link>
        </li>
        <li className={styles.sidebarItem}>
          <Link to="/app/bundles">bundles</Link>
        </li>
        <li className={styles.sidebarItem}>
          <Link to="/app/treemap">treemap</Link>
        </li>
        <li className={styles.sidebarItem}>
          <Link to="/app/foamtreemap">foamtreemap</Link>
        </li>
      </ul>

      <CacheKeyList
        isLoading={isLoading}
        error={error}
        cacheKeys={cacheKeys}
        sortBy={sortBy}
        setSortBy={setSortBy}
      />
    </div>
  );
}
