import './globals.css';
import {Outlet} from 'react-router';
import {Suspense} from 'react';
import styles from './App.module.css';
import {Sidebar} from './Sidebar';
import {DefaultLoadingIndicator} from './DefaultLoadingIndicator';

export default function App() {
  return (
    <div className={styles.app}>
      <Sidebar />

      <div className={styles.content}>
        <div className={styles.contentInner}>
          <Suspense fallback={<DefaultLoadingIndicator />}>
            {/* @ts-ignore */}
            <Outlet />
          </Suspense>
        </div>
      </div>
    </div>
  );
}
