import './globals.css';
import {Outlet} from 'react-router';
import {Suspense} from 'react';
import styles from './App.module.css';
import {Sidebar} from './ui/Sidebar';
import {DefaultLoadingIndicator} from './ui/DefaultLoadingIndicator';

export default function App() {
  return (
    <div className={styles.app}>
      <Sidebar />

      <div className={styles.content}>
        <div className={styles.contentInner}>
          <Suspense fallback={<DefaultLoadingIndicator />}>
            <Outlet />
          </Suspense>
        </div>
      </div>
    </div>
  );
}
