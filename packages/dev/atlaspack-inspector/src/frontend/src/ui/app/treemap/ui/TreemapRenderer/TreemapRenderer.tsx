import {useCallback, useEffect, useRef} from 'react';
// @ts-expect-error
import CarrotSearchFoamTree from '@carrotsearch/foamtree';
import {useSuspenseQuery} from '@tanstack/react-query';
import {AssetTreeNode, Bundle} from '../Treemap';
import {SetURLSearchParams, useSearchParams} from 'react-router';
import qs from 'qs';
import {autorun, runInAction} from 'mobx';
import {observer} from 'mobx-react-lite';
import {BundleData, Group, viewModel} from '../../../../model/ViewModel';
import {TreemapTooltip} from './TreemapTooltip';
import * as styles from './TreemapRenderer.module.css';
import {useStableCallback} from './controllers/useStableCallback';
import {useMouseMoveController} from './useMouseMoveController';

function setup(
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
          setSearchParams((prev) => {
            prev.set('focusedBundleId', e.group.id);
            prev.delete('focusedGroupId');
            return prev;
          });
        }
      } else if (e.group) {
        const focusGroup = e.group;
        this.open(focusGroup);
        this.zoom(focusGroup);

        setSearchParams((prev) => {
          prev.set('focusedGroupId', focusGroup.id);
          return prev;
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

function toBundleData(bundles: Array<Bundle>): BundleData {
  function assetTreeToGroup(assetTree: AssetTreeNode): Group {
    return {
      id: assetTree.id,
      type: 'asset',
      label: assetTree.path,
      weight: assetTree.size,
      groups: Object.entries(assetTree.children).map(([_key, child]) =>
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
        assetTreeSize: bundle.assetTree.size,
        groups: Object.entries(bundle.assetTree.children).map(([_key, child]) =>
          assetTreeToGroup(child),
        ),
      };
    }),
  };
}

export const TreemapRenderer = observer(() => {
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
  const setSearchParamsMemo = useStableCallback(setSearchParams);
  const bundle = searchParams.get('bundle');
  const foamtreeRef = useRef<CarrotSearchFoamTree | null>(null);
  const maxLevels = Number(searchParams.get('maxLevels') ?? Infinity);
  const stacking = searchParams.get('stacking') ?? 'hierarchical';
  useEffect(() => {
    if (visualizationRef.current) {
      const {cleanup, foamtree} = setup(
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
        runInAction(() => {
          viewModel.data = bundleData;
        });
        foamtreeRef.current.set({
          dataObject: bundleData,
        });
        return;
      }

      const bundleData = toBundleData(data.bundles);
      runInAction(() => {
        viewModel.data = bundleData;
      });
      if (data.bundles.length === 1) {
        setSearchParams((prev) => {
          prev.set('focusedBundleId', bundleData.groups[0].id);
          return prev;
        });
      }
      foamtreeRef.current.set({
        dataObject: bundleData,
      });
    });
  }, [data, setSearchParams]);

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

  useMouseMoveController();

  const onMouseLeave = useCallback(() => {
    runInAction(() => {
      viewModel.tooltipState = null;
    });
  }, []);

  return (
    <div
      className={styles.treemapRenderer}
      onMouseLeave={onMouseLeave}
      onMouseOut={onMouseLeave}
    >
      <div ref={visualizationRef} className={styles.expander} />

      <TreemapTooltip />
    </div>
  );
});
