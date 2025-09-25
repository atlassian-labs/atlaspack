import {observer} from 'mobx-react-lite';
import {RelatedBundlesController} from './controllers/RelatedBundlesController';
import {BottomPanel} from './ui/BottomPanel/BottomPanel';
import {FocusBreadcrumbs} from './ui/FocusBreadcrumbs/FocusBreadcrumbs';
import {TreemapRenderer} from './ui/TreemapRenderer/TreemapRenderer';
import {UrlFocusController} from './controllers/UrlFocusController';

import * as styles from './FoamTreemapPage.module.css';
import {viewModel} from '../../model/ViewModel';
import {BottomPanelResizeState} from './BottomPanelResizeState';

const bottomPanelResizeState = new BottomPanelResizeState(viewModel);

export const FoamTreemapPage = observer(() => {
  return (
    <div className={styles.foamTreemapPage}>
      <FocusBreadcrumbs />

      <div className={styles.foamTreemapPageRenderer}>
        <TreemapRenderer />
      </div>

      <div
        className={styles.foamTreemapPageBottomPanel}
        style={{
          height: `${viewModel.bottomPanelHeight}px`,
          borderColor: bottomPanelResizeState.isHovering
            ? 'var(--ds-border-accent-blue)'
            : 'var(--ds-border)',
        }}
      >
        <div
          className={styles.foamTreemapPageBottomPanelResizeHandle}
          onMouseDown={bottomPanelResizeState.startResize}
          onMouseUp={bottomPanelResizeState.stopResize}
          onMouseEnter={bottomPanelResizeState.mouseEnter}
          onMouseLeave={bottomPanelResizeState.mouseLeave}
        />

        <BottomPanel />
      </div>

      <RelatedBundlesController />
      <UrlFocusController />
    </div>
  );
});
