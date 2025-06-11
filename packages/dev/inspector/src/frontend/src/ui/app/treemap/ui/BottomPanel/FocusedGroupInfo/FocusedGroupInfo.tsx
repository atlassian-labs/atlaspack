import {useSearchParams} from 'react-router';
import {observer} from 'mobx-react-lite';
import Tabs, {Tab, TabList, TabPanel} from '@atlaskit/tabs';
import {Group, viewModel} from '../../../../../model/ViewModel';
import {GraphContainer} from './GraphContainer';
import {BundleGraphRenderer} from '../../BundleGraphRenderer';
import {AdvancedSettings} from './AdvancedSettings';
import {useSuspenseQuery} from '@tanstack/react-query';
import {useMemo} from 'react';
import {Graph} from '../../../../../types/Graph';
import {formatBytes} from '../../../../../util/formatBytes';
import {SigmaGraph} from '../../SigmaGraph';
import {AssetTable} from './AssetTable/AssetTable';
import qs from 'qs';

const FocusedGroupInfoInner = observer(
  ({group, bundle}: {group: Group; bundle: string}) => {
    console.log('group', group);
    console.log('bundle', bundle);

    const {data} = useSuspenseQuery<{
      relevantPaths: string[][];
      importers: string[];
    }>({
      queryKey: [
        '/api/treemap/reasons?' +
          qs.stringify({
            path: group.id,
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
      <div
        style={{
          display: 'flex',
          flexDirection: 'row',
          gap: '8px',
          width: '100%',
          height: '100%',
        }}
      >
        <Tabs id="focused-group-info-tabs">
          <TabList>
            <Tab>Bottom-up</Tab>
            <Tab>Top-down</Tab>
            <Tab>Advanced settings</Tab>
          </TabList>

          <TabPanel>
            <div
              style={{
                overflowX: 'hidden',
                overflowY: 'auto',
                height: 'calc(100% - 8px)',
                width: '100%',
              }}
            >
              <AssetTable data={data} isBottomUp />
            </div>
          </TabPanel>

          <TabPanel>
            <div
              style={{
                overflowX: 'hidden',
                overflowY: 'auto',
                height: 'calc(100% - 8px)',
                width: '100%',
              }}
            >
              <AssetTable data={data} isBottomUp={false} />
            </div>
          </TabPanel>

          <TabPanel>
            <div
              style={{
                overflowX: 'hidden',
                overflowY: 'auto',
                height: 'calc(100% - 8px)',
                width: '100%',
              }}
            >
              <AdvancedSettings />
            </div>
          </TabPanel>
        </Tabs>

        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '8px',
            height: '100%',
            width: 400,
          }}
        >
          <div style={{display: 'flex', flexDirection: 'column', gap: '4px'}}>
            <strong>{viewModel.focusedGroup!.label}</strong>
            {formatBytes(viewModel.focusedGroup!.weight)}
          </div>

          <GraphContainer>
            <SigmaGraph graph={graph} />
          </GraphContainer>
        </div>
      </div>
    );
  },
);

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
          <GraphContainer fullWidth>
            <BundleGraphRenderer />
          </GraphContainer>
        </TabPanel>

        <TabPanel>
          <div
            style={{
              overflowX: 'hidden',
              overflowY: 'auto',
              height: 'calc(100% - 8px)',
              width: '100%',
            }}
          >
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
