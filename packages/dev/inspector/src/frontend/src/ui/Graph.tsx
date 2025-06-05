export interface Node<T = null> {
  id: string;
  nodeId: string;
  path: string[] | null;
  displayName: string;
  level: number;
  edges: string[];
  extra: T;
}
export interface Graph<T = null> {
  nodes: Node<T>[];
}
