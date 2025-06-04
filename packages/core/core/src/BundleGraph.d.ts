import type {Node} from './types';

export interface BundleGraph {
  _graph: {
    nodes: Node[];
    getNode: (id: string) => Node;
    getNodeIdByContentKey: (key: string) => string;
    getNodeIdsConnectedFrom: (id: string, types: string[]) => string[];
    dfs: (options: any) => void;
    traverse: (options: any, startId: string, types: string[]) => void;
    rootNodeId: string;
  };
  deserialize(value: any): BundleGraph;
}

export default BundleGraph;
