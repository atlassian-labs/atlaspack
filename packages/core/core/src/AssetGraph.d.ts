import type {Node} from './types';

export default class AssetGraph {
  getNode(id: string): Node;
  getNodeIdByContentKey(key: string): string;
  deserialize(value: any): AssetGraph;
}
