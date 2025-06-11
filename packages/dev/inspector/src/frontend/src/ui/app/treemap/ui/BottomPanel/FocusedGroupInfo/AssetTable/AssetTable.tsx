import {observer} from 'mobx-react-lite';
import {useMemo} from 'react';
import {makeAutoObservable} from 'mobx';
import {
  CollapsibleTableModel,
  CollapsibleTableNode,
} from './CollapsibleTable/CollapsibleTableModel';
import {CollapsibleTable} from './CollapsibleTable/CollapsibleTable';

export const AssetTable = observer(
  ({
    data,
    isBottomUp,
  }: {
    data: {relevantPaths: string[][]};
    isBottomUp: boolean;
  }) => {
    const model: CollapsibleTableModel = useMemo(() => {
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
      const model: CollapsibleTableModel = makeAutoObservable({
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

          const root: CollapsibleTableNode = makeAutoObservable({
            id: node[0],
            path: node[0],
            isExpanded: false,
            children: [],
            parent: null,
            level: 0,
          });
          let current: CollapsibleTableNode = root;

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
