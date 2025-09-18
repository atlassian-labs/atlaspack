import {observer} from 'mobx-react-lite';
import Tabs, {Tab, TabList, TabPanel} from '@atlaskit/tabs';
import {Group, viewModel} from '../../../../../model/ViewModel';
import {GraphContainer} from './GraphContainer';
import {AdvancedSettings} from './AdvancedSettings';
import {useSuspenseQuery} from '@tanstack/react-query';
import {useMemo} from 'react';
import {Graph} from '../../../../../types/Graph';
import {formatBytes} from '../../../../../util/formatBytes';
import {SigmaGraph} from '../../SigmaGraph';
import {AssetTable} from './AssetTable/AssetTable';
import qs from 'qs';
import {Stack} from '@atlaskit/primitives';

import * as styles from './FocusedGroupInfoInner.module.css';
import {SourceCodeURL} from './SourceCodeURL';

export const FocusedGroupInfoInner = observer(
  ({group, bundle}: {group: Group; bundle: string}) => {
    const {data} = useSuspenseQuery<{
      relevantPaths: string[][];
      sourceCodeURL: SourceCodeURL | null;
      projectRoot: string;
      repositoryRoot: string;
    }>({
      queryKey: [
        '/api/treemap/reasons?' +
          qs.stringify({
            path:
              group.type === 'asset' ? group.id.split('::')[1].slice(1) : '',
            bundle,
          }),
      ],
    });

    const graph = useMemo(() => {
      const graph: Graph<any> = {
        nodes: [],
      };

      for (const path of data.relevantPaths) {
        for (let i = 0; i < path.length; i++) {
          const node = {
            id: path[i],
            nodeId: path[i],
            displayName: path[i],
            path: path,
            level: i,
            edges: i < path.length - 1 ? [path[i + 1]] : [],
            extra: null,
          };
          graph.nodes.push(node);
        }
      }

      return graph;
    }, [data]);

    return (
      <div className={styles.focusedGroupInfoInner}>
        <Tabs id="focused-group-info-tabs">
          <TabList>
            <Tab>Bottom-up</Tab>
            <Tab>Top-down</Tab>
            <Tab>Advanced settings</Tab>
          </TabList>

          <TabPanel>
            <div className={styles.focusedGroupInfoInnerAssetTable}>
              <AssetTable data={data} isBottomUp />
            </div>
          </TabPanel>

          <TabPanel>
            <div className={styles.focusedGroupInfoInnerAssetTable}>
              <AssetTable data={data} isBottomUp={false} />
            </div>
          </TabPanel>

          <TabPanel>
            <div className={styles.focusedGroupInfoInnerAdvancedSettings}>
              <AdvancedSettings />
            </div>
          </TabPanel>
        </Tabs>

        <div className={styles.focusedGroupInfoInnerGraphContainer}>
          <Stack space="space.100" grow="fill">
            <Stack space="space.050">
              <strong>{viewModel.focusedGroup!.label}</strong>
              {formatBytes(
                viewModel.focusedGroup?.assetTreeSize ??
                  viewModel.focusedGroup!.weight,
              )}
            </Stack>

            <Stack grow="fill">
              <GraphContainer>
                <SigmaGraph graph={graph} />
              </GraphContainer>
            </Stack>
          </Stack>
        </div>
      </div>
    );
  },
);
