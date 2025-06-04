import './globals.css';
import {Outlet} from 'react-router';
import styles from './App.module.css';
import {Sidebar} from './Sidebar';

export default function App() {
  return (
    <div className={styles.app}>
      <Sidebar />

      <div className={styles.content}>
        <div className={styles.contentInner}>
          <Outlet />
        </div>
      </div>
    </div>
  );
}
