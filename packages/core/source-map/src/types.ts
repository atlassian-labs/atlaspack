export interface MappingPosition {
  line: number;
  column: number;
}

export interface IndexedMapping<T> {
  generated: MappingPosition;
  original?: MappingPosition;
  source?: T;
  name?: T;
}

export interface ParsedMap {
  sources: Array<string>;
  names: Array<string>;
  mappings: Array<IndexedMapping<number>>;
  sourcesContent: Array<string | null>;
}

export interface VLQMap {
  sources: readonly string[];
  sourcesContent?: readonly (string | null)[];
  names: readonly string[];
  mappings: string;
  version?: number;
  file?: string;
  sourceRoot?: string;
}

export interface SourceMapStringifyOptions {
  file?: string;
  sourceRoot?: string;
  inlineSources?: boolean;
  fs?: {readFile(path: string, encoding: string): Promise<string>};
  format?: 'inline' | 'string' | 'object';
  /**
   * @private
   */
  rootDir?: string;
}

export interface GenerateEmptyMapOptions {
  projectRoot: string;
  sourceName: string;
  sourceContent: string;
  lineOffset?: number;
}
