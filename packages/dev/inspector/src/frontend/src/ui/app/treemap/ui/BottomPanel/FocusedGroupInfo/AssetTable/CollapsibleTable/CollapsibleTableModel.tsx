export interface CollapsibleTableModel {
  nodes: CollapsibleTableNode[];
  focusedNodeId: string | null;
  flatNodeList: CollapsibleTableNode[];
}

export interface CollapsibleTableNode {
  id: string;
  path: string;
  sourceCodeUrl: string;
  isExpanded: boolean;
  children: CollapsibleTableNode[];
  parent: string | null;
  level: number;
}