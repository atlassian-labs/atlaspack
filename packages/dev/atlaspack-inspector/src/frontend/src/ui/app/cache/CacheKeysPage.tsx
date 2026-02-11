import {Outlet} from 'react-router';
import {Box} from '@atlaskit/primitives';
import {Suspense} from 'react';

import {CacheKeyList} from './ui/CacheKeyList';
import {DefaultLoadingIndicator} from '../../DefaultLoadingIndicator/DefaultLoadingIndicator';
import * as styles from './CacheKeysPage.module.css';

export function CacheKeysPage() {
  return (
    <div className={styles.cacheKeysPage}>
      <CacheKeyList />

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
