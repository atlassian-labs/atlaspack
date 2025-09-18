import {CacheFileList} from './ui/CacheFileList';
import * as styles from '../cache/CacheKeysPage.module.css';
import {Box} from '@atlaskit/primitives';
import {DefaultLoadingIndicator} from '../../DefaultLoadingIndicator/DefaultLoadingIndicator';
import {Suspense} from 'react';
import {Outlet} from 'react-router';

export function CacheInvalidationPage() {
  return (
    <div className={styles.cacheKeysPage}>
      <CacheFileList />

      <div className={styles.cacheKeysPageChild}>
        <Box padding="space.200">
          <Suspense fallback={<DefaultLoadingIndicator />}>
            <Outlet />
          </Suspense>
        </Box>
      </div>
    </div>
  );
}
