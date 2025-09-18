import {Suspense} from 'react';
import Spinner from '@atlaskit/spinner';
import {FocusedGroupInfo} from './FocusedGroupInfo/FocusedGroupInfo';

import * as styles from './BottomPanel.module.css';

export function BottomPanel() {
  return (
    <div className={styles.bottomPanel} onClick={(e) => e.stopPropagation()}>
      <div className={styles.bottomPanelInner}>
        <Suspense
          fallback={
            <div className={styles.loadingIndicator}>
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
