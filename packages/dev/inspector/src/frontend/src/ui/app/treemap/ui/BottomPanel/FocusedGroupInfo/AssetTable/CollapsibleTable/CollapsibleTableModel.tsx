export interface CollapsibleTableModel {
  nodes: CollapsibleTableNode[];
  focusedNodeId: string | null;
  flatNodeList: CollapsibleTableNode[];
}

export interface CollapsibleTableNode {
  id: string;
  path: string;
  sourceCodeUrl: {
    url: string;
    type: 'github' | 'bitbucket';
  } | null;
  isExpanded: boolean;
  children: CollapsibleTableNode[];
  parent: string | null;
  level: number;
}
