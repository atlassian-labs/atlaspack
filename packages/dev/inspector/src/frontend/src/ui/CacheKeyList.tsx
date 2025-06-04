import {Link, useSearchParams} from 'react-router';
import styles from './App.module.css';

export function CacheKeyList({
  cacheKeys,
  sortBy,
  setSortBy,
  isLoading,
  error,
}: {
  cacheKeys: {keys: string[]} | undefined;
  sortBy: string;
  setSortBy: (value: string) => void;
  isLoading: boolean;
  error: Error | null;
}) {
  const [searchParams] = useSearchParams();

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (error) {
    return <div>Error: {error.message}</div>;
  }

  if (!cacheKeys) {
    throw new Error('No cache keys');
  }

  return (
    <>
      <div className={styles.sidebarFilter}>
        <label>Sort by</label>
        <select value={sortBy} onChange={(e) => setSortBy(e.target.value)}>
          <option value="order">order</option>
          <option value="size">size</option>
        </select>
      </div>

      <ul>
        {cacheKeys.keys.map((key) => (
          <li key={key} className={styles.sidebarItem} title={key}>
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
