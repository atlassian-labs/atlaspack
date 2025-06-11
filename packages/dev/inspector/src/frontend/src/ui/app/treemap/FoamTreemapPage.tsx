import {observer} from 'mobx-react-lite';
import {RelatedBundlesController} from './controllers/RelatedBundlesController';
import {BottomPanel} from './ui/BottomPanel/BottomPanel';
import {FocusBreadcrumbs} from './ui/FocusBreadcrumbs/FocusBreadcrumbs';
import {TreemapRenderer} from './ui/TreemapRenderer/TreemapRenderer';

export const FoamTreemapPage = observer(() => {
  return (
    <div
      style={{
        height: '100%',
        width: '100%',
        display: 'flex',
        flexDirection: 'column',
      }}
    >
      <FocusBreadcrumbs />

      <div style={{flex: 1}}>
        <TreemapRenderer />
      </div>

      <div
        style={{
          height: 400,
          borderTop: '1px solid var(--ds-border)',
        }}
      >
        <BottomPanel />
      </div>

      <RelatedBundlesController />
    </div>
  );
});
