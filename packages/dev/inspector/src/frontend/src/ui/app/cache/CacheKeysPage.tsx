import {Outlet} from 'react-router';
import {Box} from '@atlaskit/primitives';
import {Suspense} from 'react';

import {CacheKeyList} from './ui/CacheKeyList';
import {DefaultLoadingIndicator} from '../../ui/DefaultLoadingIndicator';

export function CacheKeysPage() {
  return (
    <div
      style={{
        display: 'flex',
        height: 'calc(100vh - 48px)',
        width: '100%',
      }}
    >
      <CacheKeyList />

      <div style={{overflow: 'auto', flex: 1, maxHeight: '100%'}}>
        <Box padding="space.200">
          <Suspense fallback={<DefaultLoadingIndicator />}>
            <Outlet />
          </Suspense>
        </Box>
      </div>
    </div>
  );
}
