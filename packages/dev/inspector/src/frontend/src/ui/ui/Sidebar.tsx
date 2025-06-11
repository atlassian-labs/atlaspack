import { Link } from 'react-router';
import styles from './App.module.css';
import { CacheKeyList } from '../app/cache/ui/CacheKeyList';

export function Sidebar() {
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
          <Link to="/app/treemap">foamtreemap</Link>
        </li>
      </ul>

      <CacheKeyList />
    </div>
  );
}
