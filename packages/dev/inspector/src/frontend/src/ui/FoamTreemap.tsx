import {useEffect, useRef, useState} from 'react';
// @ts-ignore
import CarrotSearchFoamTree from '@carrotsearch/foamtree';
import {useQuery} from '@tanstack/react-query';
import {AssetTreeNode, Bundle} from './Treemap';
import {formatBytes} from './formatBytes';

interface BundleData {
  groups: Array<Group>;
}

interface Group {
  label: string;
  weight: number;
  groups?: Array<Group>;
}

interface TooltipState {
  group: Group;
}

function setup(
  bundleData: BundleData,
  visualization: HTMLDivElement,
  setTooltipState: (state: TooltipState | null) => void,
) {
  // Foam Tree docs:
  // https://get.carrotsearch.com/foamtree/demo/api/index.html
  // Some options from Atlaspack 1 Visualizer:
  // https://github.com/gregtillbrook/parcel-plugin-bundle-visualiser/blob/ca5440fc61c85e40e7abc220ad99e274c7c104c6/src/buildReportAssets/init.js#L4
  // and Webpack Bundle Analyzer:
  // https://github.com/webpack-contrib/webpack-bundle-analyzer/blob/4a232f0cf7bbfed907a5c554879edd5d6f4b48ce/client/components/Treemap.jsx
  let foamtree = new CarrotSearchFoamTree({
    element: visualization,
    dataObject: bundleData,
    layout: 'squarified',
    stacking: 'flattened',
    pixelRatio: window.devicePixelRatio || 1,
    maxGroups: Infinity,
    maxGroupLevelsDrawn: Infinity,
    maxGroupLabelLevelsDrawn: Infinity,
    maxGroupLevelsAttached: Infinity,
    rolloutDuration: 0,
    pullbackDuration: 0,
    maxLabelSizeForTitleBar: 0, // disable the title bar
    onGroupHover(e: {group: Group; xAbsolute: number; yAbsolute: number}) {
      if (e.group.label == null || e.group.weight == null) {
        setTooltipState(null);
        return;
      }

      setTooltipState({
        group: e.group,
      });
    },
    onGroupClick(e: {group: Group}) {
      this.zoom(e.group);
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

  return () => {
    window.removeEventListener('resize', onResize);
    foamtree.dispose();
  };
}

function toBundleData(bundles: Array<Bundle>): BundleData {
  function assetTreeToGroup(assetTree: AssetTreeNode): Group {
    return {
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
        label: bundle.displayName,
        weight: bundle.size,
        groups: Object.entries(bundle.assetTree.children).map(([key, child]) =>
          assetTreeToGroup(child),
        ),
      };
    }),
  };
}

export function FoamTreemap() {
  const {data, isLoading, error} = useQuery<{
    bundles: Array<Bundle>;
    totalSize: number;
  }>({
    queryKey: ['/api/treemap'],
  });
  const visualizationRef = useRef<HTMLDivElement>(null);
  const [tooltipState, setTooltipState] = useState<TooltipState | null>(null);
  const [mouseState, setMouseState] = useState<{x: number; y: number}>({
    x: 0,
    y: 0,
  });

  useEffect(() => {
    const onMouseMove = (e: MouseEvent) => {
      setMouseState({x: e.clientX, y: e.clientY});
    };
    window.addEventListener('mousemove', onMouseMove);
    return () => window.removeEventListener('mousemove', onMouseMove);
  }, []);

  useEffect(() => {
    if (data && visualizationRef.current) {
      return setup(
        toBundleData(data.bundles),
        visualizationRef.current,
        setTooltipState,
      );
    }
  }, [data]);

  if (isLoading) {
    return <div>Loading...</div>;
  }
  if (error) {
    return <div>Error: {error.message}</div>;
  }

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
