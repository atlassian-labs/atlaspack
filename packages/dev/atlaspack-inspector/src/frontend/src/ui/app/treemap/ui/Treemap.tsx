export type AssetTreeNode = {
  id: string;
  children: Record<string, AssetTreeNode>;
  size: number;
  path: string;
};

export interface Bundle {
  id: string;
  size: number;
  displayName: string;
  filePath: string;
  assetTree: AssetTreeNode;
}
