export interface Node {
  type: string;
  id: string;
  value: any;
}

export interface CacheResult {
  assetGraph?: any;
  bundleGraph?: any;
}
