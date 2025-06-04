export interface Node {
  id: string;
  nodeId: string;
  path: string[] | null;
  displayName: string;
  level: number;

  edges: string[];
}
export interface Graph {
  nodes: Node[];
}
