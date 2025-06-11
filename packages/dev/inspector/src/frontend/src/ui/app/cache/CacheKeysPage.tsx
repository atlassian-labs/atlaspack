import {Outlet} from 'react-router';
import {CacheKeyList} from './ui/CacheKeyList';
import {Box} from '@atlaskit/primitives';

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
          <Outlet />
        </Box>
      </div>
    </div>
  );
}
