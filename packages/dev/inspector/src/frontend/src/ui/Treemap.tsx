import {useRef, useEffect} from 'react';
import {useQuery} from '@tanstack/react-query';
import {formatBytes} from './formatBytes';
import {getRandomDarkerColor} from './getRandomDarkerColor';

interface InputNodeData {
  size: number;
}

interface DrawNode<T extends InputNodeData> {
  x: number;
  y: number;
  width: number;
  height: number;
  data: T;
}

function calculateNodes<T extends InputNodeData>(
  input: T[],
  rect: {
    width: number;
    height: number;
    x: number;
    y: number;
  },
  allowSwitching: boolean = true,
): DrawNode<T>[] {
  const totalSize = input.reduce((acc, node) => acc + node.size, 0);
  const sortedInput = input.slice().sort((a, b) => b.size - a.size);
  const nodes = [];
  let currentRect = rect;
  const totalArea = currentRect.width * currentRect.height;

  const isHorizontal = currentRect.width < currentRect.height;
  let currentRow: T[] = [];
  const rows = [currentRow];

  function scoreRow(row: T[]) {
    const rowWidth = currentRect.width;
    const rowSize = row.reduce((acc, node) => acc + node.size, 0);
    const rowArea = (rowSize / totalSize) * totalArea;
    const rowHeight = rowArea / rowWidth;
    const nodeWidths = row.map((node) => (node.size / rowSize) * rowWidth);
    const nodeAspectRatios = nodeWidths.map((width) =>
      width > rowHeight ? width / rowHeight : rowHeight / width,
    );

    const score = nodeAspectRatios.reduce(
      (acc, ratio) => Math.max(acc, ratio),
      0,
    );
    return score;
  }

  while (sortedInput.length > 0) {
    const inputNode = sortedInput.shift();
    if (!inputNode) {
      continue;
    }

    const currentRowScore = scoreRow(currentRow);
    const newRowScore = scoreRow([...currentRow, inputNode]);
    console.log({inputNode, rows, currentRow, currentRowScore, newRowScore});

    if (newRowScore < currentRowScore) {
      currentRow.push(inputNode);
    } else {
      console.log('new row for', inputNode);
      currentRow = [inputNode];
      rows.push(currentRow);
    }
  }

  for (const row of rows) {
    const rowSize = row.reduce((acc, node) => acc + node.size, 0);
    const rowArea = (rowSize / totalSize) * totalArea;
    const rowWidth = currentRect.width;
    const rowHeight = rowArea / currentRect.width;

    let offsetX = 0;

    for (const inputNode of row) {
      const node: DrawNode<T> = {
        data: inputNode,
        x: 0,
        y: 0,
        width: 0,
        height: 0,
      };
      node.width = rowWidth * (inputNode.size / rowSize);
      node.height = rowHeight;
      node.x = currentRect.x + offsetX;
      node.y = currentRect.y;
      nodes.push(node);

      offsetX += node.width;
    }

    currentRect.y += rowHeight;
    currentRect.height -= rowHeight;
  }

  return nodes;
}

export type AssetTreeNode = {
  children: Record<string, AssetTreeNode>;
  size: number;
  path: string;
};

type AssetTreeDrawNode = DrawNode<{
  self: AssetTreeNode;
  size: number;
  children: AssetTreeDrawNode[];
}>;

function calculateAssetNodes(
  assetTree: AssetTreeNode[],
  rect: {
    width: number;
    height: number;
    x: number;
    y: number;
  },
  margin: number,
): AssetTreeDrawNode[] {
  const inputs = Object.values(assetTree).map((node) => ({
    self: node,
    children: [] as AssetTreeDrawNode[],
    size: node.size,
  }));

  const nodes: AssetTreeDrawNode[] = calculateNodes(inputs, rect, false);

  const result: AssetTreeDrawNode[] = nodes.map((node) => {
    const data: AssetTreeDrawNode['data'] = {
      self: node.data.self,
      size: node.data.size,
      children: calculateAssetNodes(
        Object.values(node.data.self.children),
        {
          width: node.width - margin * 2,
          height: node.height - margin * 2,
          x: node.x + margin,
          y: node.y + margin,
        },
        margin,
      ),
    };

    return {
      data,
      x: node.x + margin,
      y: node.y + margin,
      width: node.width - margin * 2,
      height: node.height - margin * 2,
    };
  });

  return result;
}

export interface Bundle {
  id: string;
  size: number;
  displayName: string;
  filePath: string;
  assetTree: AssetTreeNode;
}

export function Treemap() {
  const {data, isLoading, error} = useQuery<{
    bundles: Array<Bundle>;
    totalSize: number;
  }>({
    queryKey: ['/api/treemap'],
  });
  const treemapRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  // const [viewModel, setViewModel] = useState<{
  //   width: number;
  //   height: number;
  //   nodes: {
  //     id: string;
  //     size: number;
  //     ratio: number;
  //     x: number;
  //     y: number;
  //     width: number;
  //     height: number;
  //     displayName: string;
  //     color: {
  //       familyName: string;
  //       family: string[];
  //     };
  //   }[];
  // }>({
  //   width: 0,
  //   height: 0,
  //   nodes: [],
  // });

  useEffect(() => {
    function render() {
      if (!data) {
        return;
      }

      if (!treemapRef.current) {
        return;
      }

      const treemap = treemapRef.current;
      const treemapRect = treemap.getBoundingClientRect();

      if (!canvasRef.current) {
        return;
      }

      const context: CanvasRenderingContext2D | null =
        canvasRef.current?.getContext('2d') ?? null;
      if (!context) {
        return;
      }

      const devicePixelRatio = window.devicePixelRatio;
      const width = treemapRect.width * devicePixelRatio;
      const height = treemapRect.height * devicePixelRatio;
      canvasRef.current.width = width;
      canvasRef.current.height = height;
      canvasRef.current.style.width = treemapRect.width + 'px';
      canvasRef.current.style.height = treemapRect.height + 'px';
      context.scale(devicePixelRatio, devicePixelRatio);

      const currentRect = {
        width: treemapRect.width,
        height: treemapRect.height,
        x: 0,
        y: 0,
      };

      const nodes = calculateNodes(data.bundles.slice(4, 5), currentRect);

      if (context) {
        const margin = 10;
        const borderRadius = 4;

        // Draw rects
        for (const node of nodes) {
          const bundleColor = getRandomDarkerColor(node.data.displayName);
          context.fillStyle = bundleColor.family[1];
          context.beginPath();
          const bounds = {
            x: node.x + margin,
            y: node.y + margin,
            width: node.width - margin * 2,
            height: node.height - margin * 2,
          };
          context.roundRect(
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            borderRadius,
          );
          context.fill();

          const textHeight = 40;
          context.fillStyle = 'black';
          context.font = '20px sans-serif';
          context.fillText(
            node.data.displayName + ' ' + formatBytes(node.data.size),
            node.x + margin * 2,
            node.y + margin * 4,
            node.width - margin * 4,
          );

          const assetNodes = calculateAssetNodes(
            Object.values(node.data.assetTree.children),
            {
              width: bounds.width,
              height: bounds.height - textHeight - margin,
              x: bounds.x,
              y: bounds.y + textHeight,
            },
            margin / 4,
          );

          const drawAssetNode = (
            assetNode: AssetTreeDrawNode,
            level: number = 2,
            parent: {
              x: number;
              y: number;
              width: number;
              height: number;
            },
          ) => {
            if (level > 3) {
              return;
            }

            context.fillStyle = bundleColor.family[level];
            context.beginPath();

            context.strokeStyle = 'black';
            context.lineWidth = 1;
            context.roundRect(
              assetNode.x,
              assetNode.y,
              assetNode.width,
              assetNode.height,
              borderRadius,
            );
            context.stroke();
            context.fill();
            context.fillStyle = 'black';
            context.font = '8px sans-serif';
            context.fillText(
              `${assetNode.data.self.path} (${formatBytes(
                assetNode.data.size,
              )})`,
              assetNode.x,
              assetNode.y + 10,
              assetNode.width,
            );

            for (const child of assetNode.data.children) {
              drawAssetNode(child, level + 1, assetNode);
            }
          };

          for (const assetNode of assetNodes) {
            drawAssetNode(assetNode, 2, bounds);
          }
        }
      }

      // setViewModel({
      //   width: treemapRect.width,
      //   height: treemapRect.height,
      //   nodes,
      // });
    }

    render();

    window.addEventListener('resize', render);
    return () => window.removeEventListener('resize', render);
  }, [treemapRef, data, isLoading]);

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (error) {
    return <div>Error: {error.message}</div>;
  }

  return (
    <div
      ref={treemapRef}
      style={{width: '100%', flex: 1, height: '100%', position: 'relative'}}
    >
      {/* {viewModel.nodes.map((node) => (
        <div
          key={node.id}
          style={{
            position: 'absolute',
            backgroundColor: 'red',
            left: node.x,
            top: node.y,
            width: node.width,
            height: node.height,
          }}
        >
          <pre>{node.displayName}</pre>
        </div>
      ))} */}
      <canvas ref={canvasRef} />
    </div>
  );
}
