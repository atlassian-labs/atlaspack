import {Suspense} from 'react';
import Spinner from '@atlaskit/spinner';
import {FocusedGroupInfo} from './FocusedGroupInfo/FocusedGroupInfo';

export function BottomPanel() {
  return (
    <div
      onClick={(e) => e.stopPropagation()}
      style={{
        borderLeft: '1px solid var(--border-color)',
        height: '100%',
        display: 'flex',
      }}
    >
      <div
        style={{
          display: 'flex',
          flexDirection: 'row',
          gap: '10px',
          width: '100%',
          height: '100%',
          paddingLeft: '16px',
        }}
      >
        <Suspense
          fallback={
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                flexDirection: 'column',
                gap: '16px',
                height: '100%',
                width: '100%',
              }}
            >
              <Spinner size="large" />
              <h2>Loading bundle graph data...</h2>
            </div>
          }
        >
          <FocusedGroupInfo />
        </Suspense>
      </div>
    </div>
  );
}
