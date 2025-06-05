import {Suspense, useCallback, useEffect, useRef, useState} from 'react';
// @ts-ignore
import CarrotSearchFoamTree from '@carrotsearch/foamtree';
import {useQuery, useSuspenseQuery} from '@tanstack/react-query';
import {AssetTreeNode, Bundle} from './Treemap';
import {formatBytes} from './formatBytes';
import {SetURLSearchParams, useSearchParams} from 'react-router';
import qs from 'qs';
import {autorun, makeAutoObservable, runInAction} from 'mobx';
import {observer} from 'mobx-react-lite';
interface BundleData {
  groups: Array<Group>;
}

interface RelatedBundles {
  childBundles: Array<{id: string; displayName: string; size: number}>;
}

interface ViewModel {
  focusedBundle: Group | null;
  relatedBundles: RelatedBundles | null;
  hasDetails: boolean;
}

const viewModel: ViewModel = makeAutoObservable({
  focusedBundle: null,
  relatedBundles: null,
  hasDetails: false,
});

interface Group {
  id: string;
  type: 'bundle' | 'asset';
  label: string;
  weight: number;
  groups?: Array<Group>;
}

interface TooltipState {
  group: Group;
}

function setup(
  // bundleData: BundleData,
  visualization: HTMLDivElement,
  setTooltipState: (state: TooltipState | null) => void,
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
      if (e.group == null || e.group.label == null || e.group.weight == null) {
        setTooltipState(null);
        return;
      }

      setTooltipState({
        group: e.group,
      });
    },
    onGroupClick(e: {group: Group}) {
      if (!isDetailView) {
        if (e.group.type === 'bundle') {
          runInAction(() => {
            viewModel.focusedBundle = e.group;
          });
        }
      } else {
        this.open(e.group);
        this.zoom(e.group);
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

  window.addEventListener('resize', onResize);

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
      window.removeEventListener('resize', onResize);
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

function TreemapRenderer() {
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
  const [tooltipState, setTooltipState] = useState<TooltipState | null>(null);
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
        setTooltipState,
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
        foamtreeRef.current.set({
          dataObject: toBundleData(
            data.bundles.filter(
              (group) =>
                group.id === viewModel.focusedBundle?.id ||
                relatedBundlesSet.has(group.id),
            ),
          ),
        });
        return;
      }

      foamtreeRef.current.set({
        dataObject: toBundleData(data.bundles),
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
      onMouseLeave={() => setTooltipState(null)}
    >
      <div
        ref={visualizationRef}
        style={{height: '100%', width: '100%', flex: 1}}
      />

      {tooltipState && (
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
          {tooltipState.group.label} - {formatBytes(tooltipState.group.weight)}
        </div>
      )}
    </div>
  );
}

function AdvancedSettings() {
  const [searchParams, setSearchParams] = useSearchParams();
  const bundle = searchParams.get('bundle');
  const isDetailView = bundle != null;
  const maxLevels = searchParams.get('maxLevels') ?? 0;
  const stacking = searchParams.get('stacking') ?? 'hierarchical';

  return (
    <details>
      <summary>Advanced settings</summary>

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
    </details>
  );
}

function RightSidebar() {
  return (
    <div
      style={{
        width: '300px',
        borderLeft: '1px solid var(--border-color)',
        display: 'flex',
      }}
    >
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '10px',
          padding: '10px',
          maxWidth: '100%',
        }}
      >
        <AdvancedSettings />

        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '8px',
            height: '100%',
            width: '100%',
          }}
        >
          <strong>Bundles</strong>
          <Suspense
            fallback={
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                }}
              >
                Loading...
              </div>
            }
          >
            <BundlePicker />
          </Suspense>
        </div>
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
        viewModel.focusedBundle = null;
        viewModel.relatedBundles = null;
        viewModel.hasDetails = true;
      });
    }
  }, [searchParamsBundle]);

  return null;
});

const ViewPathBreadcrumbs = observer(() => {
  return (
    <div
      style={{
        borderBottom: '1px solid var(--border-color)',
        padding: '4px',
        display: 'flex',
        alignItems: 'center',
        gap: '4px',
      }}
    >
      <button
        onClick={() => {
          runInAction(() => {
            viewModel.focusedBundle = null;
            viewModel.relatedBundles = null;
          });
        }}
      >
        All bundles
      </button>
      <div>&gt;</div>
      {viewModel.focusedBundle ? (
        <>Focused on {viewModel.focusedBundle?.label}</>
      ) : (
        <>
          <em>Click a bundle to focus on it, double click to see all assets</em>
        </>
      )}
    </div>
  );
});

export function FoamTreemap() {
  const [, setSearchParams] = useSearchParams();

  return (
    <>
      <RelatedBundlesController />

      <div
        style={{display: 'flex', flexDirection: 'row', height: '100%'}}
        onClick={() =>
          setSearchParams((prev) => {
            prev.delete('bundle');
            return prev;
          })
        }
      >
        <div style={{flex: 1, display: 'flex', flexDirection: 'column'}}>
          <ViewPathBreadcrumbs />

          <Suspense
            fallback={
              <div
                style={{
                  height: '100%',
                  width: '100%',
                  flex: 1,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                }}
              >
                Loading...
              </div>
            }
          >
            <div
              onClick={(e) => e.stopPropagation()}
              style={{height: '100%', width: '100%', flex: 1, display: 'flex'}}
            >
              <TreemapRenderer />
            </div>
          </Suspense>
        </div>

        <RightSidebar />
      </div>
    </>
  );
}
