import {useSearchParams} from 'react-router';
import {observer} from 'mobx-react-lite';
import Tabs, {Tab, TabList, TabPanel} from '@atlaskit/tabs';
import {viewModel} from '../../../../../model/ViewModel';
import {GraphContainer} from './GraphContainer';
import {BundleGraphRenderer} from '../../BundleGraphRenderer';
import {AdvancedSettings} from './AdvancedSettings';
import styles from './FocusedGroupInfo.module.css';
import {FocusedGroupInfoInner} from './FocusedGroupInfoInner';

export const FocusedGroupInfo = observer(() => {
  const [searchParams] = useSearchParams();
  const bundle = searchParams.get('bundle');
  if (!viewModel.focusedGroup || !bundle) {
    return (
      <Tabs id="focused-group-info-tabs">
        <TabList>
          <Tab>Bundle graph</Tab>
          <Tab>Advanced settings</Tab>
        </TabList>

        <TabPanel>
          <div className={styles.focusedGroupInfoBundleGraph}>
            <GraphContainer fullWidth>
              <BundleGraphRenderer />
            </GraphContainer>
          </div>
        </TabPanel>

        <TabPanel>
          <div className={styles.focusedGroupInfoAdvancedSettings}>
            <AdvancedSettings />
          </div>
        </TabPanel>
      </Tabs>
    );
  }

  return (
    <FocusedGroupInfoInner group={viewModel.focusedGroup} bundle={bundle} />
  );
});
