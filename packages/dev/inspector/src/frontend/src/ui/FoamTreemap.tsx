import 'flexlayout-react/style/light.css';
import styles from './App.module.css';
import {
  Fragment,
  Suspense,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
// @ts-expect-error
import CarrotSearchFoamTree from '@carrotsearch/foamtree';
import {useQuery, useSuspenseQuery} from '@tanstack/react-query';
import {AssetTreeNode, Bundle} from './Treemap';
import {formatBytes} from './formatBytes';
import {Link, SetURLSearchParams, useSearchParams} from 'react-router';
import qs from 'qs';
import {autorun, makeAutoObservable, runInAction} from 'mobx';
import {observer} from 'mobx-react-lite';
import {Graph} from './Graph';
import {SigmaGraph} from './SigmaGraph';
import Spinner from '@atlaskit/spinner';
import {token} from '@atlaskit/tokens';
import Tabs, {Tab, TabList, TabPanel} from '@atlaskit/tabs';
import {SigmaPage} from './Sigma';
import {BundleData, Group, RelatedBundles, viewModel} from './ViewModel';
import {BitbucketIcon} from '@atlaskit/logo';

function setup(
  // bundleData: BundleData,
  visualization: HTMLDivElement,
  setSearchParams: SetURLSearchParams,
  isDetailView: boolean,
  maxLevels: number,
  stacking: string,
) {
  // Foam Tree docs:
  // https://get.carrotsearch.com/foamtree/demo/api/index.html
  // Some options from Atlaspack 1 Visualizer:
  // https://github.com/gregtillbrook/parcel-plugin-bundle-visualiser/blob/ca5440fc61c85e40e7abc220ad99e274c7c104c6/src/buildReportAssets/init.js#L4
  // and Webpack Bundle Analyzer:
  // https://github.com/webpack-contrib/webpack-bundle-analyzer/blob/4a232f0cf7bbfed907a5c554879edd5d6f4b48ce/client/components/Treemap.jsx
  let foamtree = new CarrotSearchFoamTree({
    element: visualization,
    // dataObject: bundleData,
    layout: 'squarified',
    stacking,
    pixelRatio: window.devicePixelRatio || 1,
    maxGroups: Infinity,
    groupLabelMinFontSize: 3,
    maxGroupLevelsDrawn: maxLevels,
    maxGroupLabelLevelsDrawn: maxLevels,
    maxGroupLevelsAttached: maxLevels,
    rolloutDuration: 0,
    pullbackDuration: 0,
    maxLabelSizeForTitleBar: 0, // disable the title bar
    onGroupHover(e: {group: Group; xAbsolute: number; yAbsolute: number}) {
      runInAction(() => {
        if (
          e.group == null ||
          e.group.label == null ||
          e.group.weight == null
        ) {
          viewModel.tooltipState = null;
          return;
        }

        viewModel.tooltipState = {
          group: e.group,
        };
      });
    },
    onGroupClick(e: {group: Group}) {
      if (!isDetailView) {
        if (e.group.type === 'bundle') {
          runInAction(() => {
            viewModel.focusedBundle = e.group;
            viewModel.focusedGroup = null;
          });
        }
      } else {
        const focusGroup = e.group;
        this.open(focusGroup);
        this.zoom(focusGroup);

        runInAction(() => {
          viewModel.focusedGroup = focusGroup;
        });
      }
    },
    onGroupDoubleClick(e: {group: Group}) {
      this.zoom(e.group);

      if (e.group.type === 'bundle') {
        setSearchParams((prev) => {
          prev.set('bundle', e.group.id);
          return prev;
        });
      }
    },
  });

  const onResize = debounce(() => {
    foamtree.resize();
  }, 100);

  const resizeObserver = new ResizeObserver(onResize);
  resizeObserver.observe(visualization);

  function debounce(fn: (...args: any[]) => void, delay: number): () => void {
    let timeout: NodeJS.Timeout | null = null;

    return function (...args: any[]) {
      if (timeout) {
        clearTimeout(timeout);
      }

      timeout = setTimeout(() => {
        fn(...args);
      }, delay);
    };
  }

  return {
    foamtree,
    cleanup: () => {
      resizeObserver.disconnect();
      foamtree.dispose();
    },
  };
}

function useStableCallback(fn: (...args: any[]) => void) {
  const ref = useRef(fn);
  useEffect(() => {
    ref.current = fn;
  }, [fn]);
  return useCallback((...args: any[]) => ref.current(...args), []);
}

function toBundleData(bundles: Array<Bundle>): BundleData {
  function assetTreeToGroup(assetTree: AssetTreeNode): Group {
    return {
      id: assetTree.path,
      type: 'asset',
      label: assetTree.path,
      weight: assetTree.size,
      groups: Object.entries(assetTree.children).map(([key, child]) =>
        assetTreeToGroup(child),
      ),
    };
  }

  return {
    groups: bundles.map((bundle) => {
      return {
        id: bundle.id,
        type: 'bundle',
        label: bundle.displayName,
        weight: bundle.size,
        groups: Object.entries(bundle.assetTree.children).map(([key, child]) =>
          assetTreeToGroup(child),
        ),
      };
    }),
  };
}

const BundlePicker = observer(() => {
  const [searchParams, setSearchParams] = useSearchParams();
  const bundle = searchParams.get('bundle');
  const {data, error} = useSuspenseQuery<{
    bundles: Array<{
      id: string;
      displayName: string;
      size: number;
      bundle: any;
    }>;
    count: number;
  }>({
    queryKey: ['/api/bundle-graph/bundles'],
  });

  if (error) {
    return <div>Error: {error.message}</div>;
  }

  const relatedBundlesSet = new Set(
    viewModel.relatedBundles?.childBundles.map((b: any) => b.id),
  );
  const bundles = viewModel.relatedBundles
    ? data.bundles.filter(
        (group) =>
          group.id === viewModel.focusedBundle?.id ||
          relatedBundlesSet.has(group.id),
      )
    : data.bundles;

  bundles.sort((a, b) => b.size - a.size);

  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        overflowY: 'auto',
        overflowX: 'hidden',
        display: 'flex',
        alignItems: 'center',
        flexDirection: 'column',
        gap: '8px',
      }}
    >
      {bundles.map((bundle) => (
        <button
          key={bundle.id}
          title={bundle.displayName}
          style={{
            alignItems: 'center',
            width: '100%',
            whiteSpace: 'nowrap',
            height: '15px',
            textOverflow: 'ellipsis',
            gap: '4px',
            justifyContent: 'flex-start',
            textAlign: 'left',
            border: 'none',
            background: 'none',
          }}
        >
          {formatBytes(bundle.size)} - {bundle.displayName}
        </button>
      ))}
    </div>
  );
});

const TreemapRenderer = observer(() => {
  const [searchParams, setSearchParams] = useSearchParams();

  const {data} = useSuspenseQuery<{
    bundles: Array<Bundle>;
    totalSize: number;
  }>({
    queryKey: [
      '/api/treemap?' +
        qs.stringify({
          offset: searchParams.get('offset') ?? 0,
          limit: searchParams.get('limit') ?? 10000,
          bundle: searchParams.get('bundle') ?? undefined,
        }),
    ],
  });
  const visualizationRef = useRef<HTMLDivElement>(null);
  const [mouseState, setMouseState] = useState<{x: number; y: number}>({
    x: 0,
    y: 0,
  });

  const setSearchParamsMemo = useStableCallback(setSearchParams);
  const bundle = searchParams.get('bundle');
  const foamtreeRef = useRef<CarrotSearchFoamTree | null>(null);
  const maxLevels = Number(searchParams.get('maxLevels') ?? Infinity);
  const stacking = searchParams.get('stacking') ?? 'hierarchical';
  useEffect(() => {
    if (visualizationRef.current) {
      const {cleanup, foamtree} = setup(
        // toBundleData(data.bundles),
        visualizationRef.current,
        setSearchParamsMemo,
        bundle != null,
        maxLevels,
        stacking,
      );
      foamtreeRef.current = foamtree;

      return () => {
        cleanup();
      };
    }
  }, [bundle, setSearchParamsMemo, maxLevels, stacking]);

  useEffect(() => {
    return autorun(() => {
      if (!foamtreeRef.current) {
        return;
      }

      if (viewModel.relatedBundles) {
        const relatedBundlesSet = new Set(
          viewModel.relatedBundles.childBundles.map((b: any) => b.id),
        );
        const bundleData = toBundleData(
          data.bundles.filter(
            (group) =>
              group.id === viewModel.focusedBundle?.id ||
              relatedBundlesSet.has(group.id),
          ),
        );
        foamtreeRef.current.set({
          dataObject: bundleData,
        });
        return;
      }

      const bundleData = toBundleData(data.bundles);
      runInAction(() => {
        if (data.bundles.length === 1) {
          viewModel.focusedBundle = bundleData.groups[0];
        }
      });
      foamtreeRef.current.set({
        dataObject: bundleData,
      });
    });
  }, [data]);

  useEffect(() => {
    return autorun(() => {
      if (viewModel.hasDetails) {
        return;
      }

      if (!viewModel.focusedBundle) {
        return;
      }
      if (!viewModel.relatedBundles) {
        return;
      }

      foamtreeRef.current?.expose([
        viewModel.focusedBundle.id,
        ...viewModel.relatedBundles.childBundles.map((b) => b.id),
      ]);
    });
  }, []);

  useEffect(() => {
    const onMouseMove = (e: MouseEvent) => {
      setMouseState({x: e.clientX, y: e.clientY});
    };
    window.addEventListener('mousemove', onMouseMove);
    return () => window.removeEventListener('mousemove', onMouseMove);
  }, []);

  return (
    <div
      style={{height: '100%', width: '100%', flex: 1}}
      onMouseLeave={() => runInAction(() => (viewModel.tooltipState = null))}
    >
      <div
        ref={visualizationRef}
        style={{height: '100%', width: '100%', flex: 1}}
      />

      {viewModel.tooltipState && (
        <div
          style={{
            position: 'absolute',
            left: mouseState.x + 10,
            top: mouseState.y + 10,
            backgroundColor: 'white',
            padding: '10px',
            borderRadius: '4px',
            boxShadow: '0 0 10px 0 rgba(0, 0, 0, 0.1)',
          }}
        >
          {viewModel.tooltipState.group.label}
          <br />
          {formatBytes(viewModel.tooltipState.group.weight)}
        </div>
      )}
    </div>
  );
});

function AdvancedSettings() {
  const [searchParams, setSearchParams] = useSearchParams();
  const bundle = searchParams.get('bundle');
  const isDetailView = bundle != null;
  const maxLevels = searchParams.get('maxLevels') ?? 0;
  const stacking = searchParams.get('stacking') ?? 'hierarchical';

  return (
    <div style={{padding: '8px'}}>
      <div style={{display: 'flex', flexDirection: 'column', gap: '10px'}}>
        <label style={{fontWeight: 'bold'}}>Max levels: {maxLevels}</label>
        <input
          disabled={!isDetailView}
          type="range"
          min={0}
          max={10}
          value={maxLevels}
          onChange={(e) =>
            setSearchParams((prev) => {
              prev.set('maxLevels', e.target.value);
              return prev;
            })
          }
        />
      </div>

      <div style={{display: 'flex', flexDirection: 'column', gap: '10px'}}>
        <label style={{fontWeight: 'bold'}}>Stacking</label>
        <select
          disabled={!isDetailView}
          value={stacking}
          onChange={(e) =>
            setSearchParams((prev) => {
              prev.set('stacking', e.target.value);
              return prev;
            })
          }
        >
          <option value="hierarchical">Hierarchical</option>
          <option value="flattened">Flattened</option>
        </select>
      </div>
    </div>
  );
}

const ItemRow = observer(({node, level = 0, model}: any) => {
  useEffect(() => {
    if (model.focusedNodeId === node.id) {
      const row = document.querySelector(
        `[data-nodeid="${model.focusedNodeId}"]`,
      );
      if (row) {
        (row as HTMLElement).focus();
      }
    }
  }, [model, node]);

  return (
    <Fragment>
      <tr
        style={{height: '20px'}}
        data-nodeid={node.id}
        tabIndex={0}
        autoFocus={model.focusedNodeId === node.id}
      >
        <td
          style={{
            verticalAlign: 'baseline',
            paddingLeft: level * 16,
            display: 'flex',
            gap: 8,
          }}
        >
          {node.children.length > 0 ? (
            <button
              onClick={() => {
                runInAction(() => {
                  node.isExpanded = !node.isExpanded;
                });
              }}
              style={{border: 'none', background: 'none', width: '16px'}}
            >
              {node.isExpanded ? '▼' : '▶'}
            </button>
          ) : (
            <span style={{width: '16px'}} />
          )}

          {node.path}
        </td>
        <td>
          <a
            href={`https://bitbucket.org/atlassian/atlassian-frontend-monorepo/src/master/jira/${node.path}`}
            target="_blank"
          >
            <BitbucketIcon size="small" />
          </a>
        </td>
      </tr>
    </Fragment>
  );
});

function limit(value: number, len: number) {
  if (value < 0) {
    return len - (Math.abs(value) % len);
  }
  return value % len;
}

const CollapsibleTable = observer(({model}: {model: AssetTableModel}) => {
  function focusOnNode(nodeId: string) {
    runInAction(() => {
      model.focusedNodeId = nodeId;
    });

    const node = document.querySelector(
      `[data-nodeid="${model.focusedNodeId}"]`,
    );
    if (node) {
      node.scrollIntoView({behavior: 'smooth', block: 'center'});
      (node as HTMLElement).focus();
    }
  }

  return (
    <div style={{width: '100%'}}>
      <table
        style={{width: '100%'}}
        className={styles.treemapTable}
        onKeyDown={(e) => {
          if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
            e.preventDefault();
            e.stopPropagation();

            runInAction(() => {
              const current = model.flatNodeList.findIndex(
                (node: any) => node.id === model.focusedNodeId,
              );
              if (current === -1) {
                focusOnNode(model.flatNodeList[0].id);
              } else {
                const increment = e.key === 'ArrowDown' ? 1 : -1;
                focusOnNode(
                  model.flatNodeList[
                    limit(current + increment, model.flatNodeList.length)
                  ].id,
                );
              }
            });
          } else if (e.key === 'ArrowRight') {
            e.preventDefault();
            e.stopPropagation();

            const current = model.flatNodeList.findIndex(
              (node: any) => node.id === model.focusedNodeId,
            );
            runInAction(() => {
              if (current !== -1) {
                model.flatNodeList[current].isExpanded = true;

                const newNode =
                  model.flatNodeList[
                    limit(current + 1, model.flatNodeList.length)
                  ];
                if (newNode.level > model.flatNodeList[current].level) {
                  focusOnNode(newNode.id);
                }
              }
            });
          } else if (e.key === 'ArrowLeft') {
            e.preventDefault();
            e.stopPropagation();

            const current = model.flatNodeList.find(
              (node: any) => node.id === model.focusedNodeId,
            );
            if (current && current.isExpanded) {
              runInAction(() => {
                current.isExpanded = false;
              });
            } else if (current && current.parent) {
              focusOnNode(current.parent);
            }
          }
        }}
        onFocus={(e) => {
          let current = e.target as HTMLElement;
          while (current && !current.getAttribute('data-nodeid')) {
            current = current.parentElement as HTMLElement;
          }

          runInAction(() => {
            model.focusedNodeId = current.getAttribute('data-nodeid');
          });
        }}
      >
        <tbody>
          {model.flatNodeList.map((node, i) => (
            <ItemRow key={i} node={node} level={node.level} model={model} />
          ))}
        </tbody>
      </table>
    </div>
  );
});

interface AssetTableModel {
  nodes: AssetTableNode[];
  focusedNodeId: string | null;
  flatNodeList: AssetTableNode[];
}

interface AssetTableNode {
  id: string;
  path: string;
  isExpanded: boolean;
  children: AssetTableNode[];
  parent: string | null;
  level: number;
}

const AssetTable = observer(
  ({
    data,
    isBottomUp,
  }: {
    data: {relevantPaths: string[][]};
    isBottomUp: boolean;
  }) => {
    const model: AssetTableModel = useMemo(() => {
      // this is horrible ; but let's just hope there aren't that many children
      // anyway things won't perform properly in that case
      const expanded = (node: any) => {
        if (node.isExpanded) {
          return [
            node,
            ...node.children.flatMap((child: any) => expanded(child)),
          ];
        }
        return [node];
      };
      const model: AssetTableModel = makeAutoObservable({
        nodes: [],
        focusedNodeId: null,
        get flatNodeList() {
          return this.nodes.flatMap((node) => {
            return expanded(node);
          });
        },
      });

      if (isBottomUp) {
        const seenRoots = new Set();
        for (let path of data.relevantPaths) {
          const node = path.slice();
          node.reverse();

          if (seenRoots.has(node[0])) {
            continue;
          }
          seenRoots.add(node[0]);

          const root: AssetTableNode = makeAutoObservable({
            id: node[0],
            path: node[0],
            isExpanded: false,
            children: [],
            parent: null,
            level: 0,
          });
          let current: AssetTableNode = root;

          for (let i = 1; i < node.length; i++) {
            const newNode = makeAutoObservable({
              id: current.id + '--->>>>' + node[i],
              path: node[i],
              isExpanded: false,
              children: [],
              parent: current.id,
              level: i,
            });
            current.children.push(newNode);
            current = newNode;
          }

          model.nodes.push(root);
        }
      } else {
        const roots = new Map();
        for (let path of data.relevantPaths) {
          const node = path.slice();

          if (!roots.has(node[0])) {
            const root = makeAutoObservable({
              id: node[0],
              path: node[0],
              isExpanded: false,
              children: [] as any[],
              parent: null,
              level: 0,
            });
            roots.set(node[0], root);
            model.nodes.push(root);
          }

          const root = roots.get(node[0]);
          let current = root;

          for (let i = 1; i < node.length; i++) {
            const existingNode = current.children.find(
              (child: any) => child.path === node[i],
            );
            if (existingNode) {
              current = existingNode as any;
              continue;
            }

            const newNode = makeAutoObservable({
              id: current.id + '--->>>>' + node[i],
              path: node[i],
              isExpanded: false,
              children: [],
              parent: current.id,
              level: i,
            });
            current.children.push(newNode);
            current = newNode;
          }
        }
      }

      return model;
    }, [data, isBottomUp]);

    return <CollapsibleTable model={model} />;
  },
);

function ImportersTable({data}: {data: {importers: string[]}}) {
  return (
    <table style={{width: '100%'}} className={styles.treemapTable}>
      <tbody>
        {data.importers.map((node, i) => (
          <ItemRow
            key={i}
            node={{
              children: [],
              id: node,
              isExpanded: false,
              level: 0,
              path: node,
              parent: null,
            }}
            level={0}
            model={{
              focusedNodeId: null,
            }}
          />
        ))}
      </tbody>
    </table>
  );
}

function GraphContainer({
  children,
  fullWidth = false,
}: {
  children: React.ReactNode;
  fullWidth?: boolean;
}) {
  return (
    <div
      style={{
        height: 'calc(100% - 16px)',
        width: fullWidth ? '100%' : 300,
        border: '1px solid var(--ds-border)',
        borderRadius: '8px',
        backgroundColor: token('elevation.surface.sunken'),
        margin: '8px',
      }}
    >
      {children}
    </div>
  );
}

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
            <Tab>Importers</Tab>
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
              <ImportersTable data={data} />
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

const FocusedGroupInfo = observer(() => {
  const [searchParams] = useSearchParams();
  const bundle = searchParams.get('bundle');
  if (!viewModel.focusedGroup || !bundle) {
    return (
      <GraphContainer fullWidth>
        <SigmaPage />
      </GraphContainer>
    );
  }

  return (
    <FocusedGroupInfoInner group={viewModel.focusedGroup} bundle={bundle} />
  );
});

function RightSidebar() {
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

const RelatedBundlesController = observer(() => {
  const [searchParams] = useSearchParams();
  const {data} = useQuery<RelatedBundles>({
    queryKey: [
      '/api/bundle-graph/related-bundles?' +
        qs.stringify({bundle: viewModel.focusedBundle?.id}),
    ],
    enabled: viewModel.focusedBundle != null,
  });

  useEffect(() => {
    if (data) {
      runInAction(() => {
        viewModel.relatedBundles = data;
      });
    }
  }, [data]);

  const searchParamsBundle = searchParams.get('bundle');
  useEffect(() => {
    if (searchParamsBundle != null) {
      runInAction(() => {
        viewModel.relatedBundles = null;
        viewModel.hasDetails = true;
      });
    }
  }, [searchParamsBundle]);

  return null;
});

const FocusBreadcrumbs = observer(() => {
  const bundleEl = viewModel.focusedBundle ? (
    <Link to={`/app/foamtreemap?bundle=${viewModel.focusedBundle.id}`}>
      {viewModel.focusedBundle.label}
    </Link>
  ) : null;
  const focusedGroup = viewModel.focusedGroup
    ? viewModel.focusedGroup.id.split('/').map((part, i, arr) => {
        const candidatePath = arr.slice(0, i + 1).join('/');
        return (
          <div key={i}>
            <Link
              to={`/app/foamtreemap?bundle=${viewModel.focusedBundle?.id}&path=${candidatePath}`}
              onClick={(e) => {
                // TODO: Make this work
                e.preventDefault();

                runInAction(() => {
                  viewModel.focusedGroup = null;
                });
              }}
            >
              {part}
            </Link>
          </div>
        );
      })
    : [];
  const breadcrumEls = [
    <Link to="/app/foamtreemap">Root</Link>,
    bundleEl,
    ...focusedGroup,
  ];

  return (
    <div
      style={{
        padding: '4px',
        display: 'flex',
        flexDirection: 'row',
        gap: '4px',
      }}
    >
      {breadcrumEls.flatMap((el, i) => [
        <div key={i}>{el}</div>,
        i < breadcrumEls.length - 1 && <div>&gt;</div>,
      ])}
    </div>
  );
});

export const FoamTreemap = observer(() => {
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
        <RightSidebar />
      </div>

      <RelatedBundlesController />
    </div>
  );
});
