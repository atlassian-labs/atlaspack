import {observer} from 'mobx-react-lite';
import {RelatedBundlesController} from './controllers/RelatedBundlesController';
import {BottomPanel} from './ui/BottomPanel/BottomPanel';
import {FocusBreadcrumbs} from './ui/FocusBreadcrumbs/FocusBreadcrumbs';
import {TreemapRenderer} from './ui/TreemapRenderer/TreemapRenderer';

import styles from './FoamTreemapPage.module.css';

export const FoamTreemapPage = observer(() => {
  return (
    <div className={styles.foamTreemapPage}>
      <FocusBreadcrumbs />

      <div className={styles.foamTreemapPageRenderer}>
        <TreemapRenderer />
      </div>

      <div className={styles.foamTreemapPageBottomPanel}>
        <BottomPanel />
      </div>

      <RelatedBundlesController />
    </div>
  );
});
