import type {VLQMap, GenerateEmptyMapOptions} from './types';
import SourceMap, {SOURCE_MAP_VERSION} from './SourceMap';
import {SourceMap as AtlaspackSourceMap} from '@atlaspack/rust';

// Re-export types for consumers
export type * from './types';

export default class NodeSourceMap extends SourceMap {
  constructor(projectRoot: string = '/', buffer?: Buffer) {
    super(projectRoot);
    this.projectRoot = projectRoot;
    this.sourceMapInstance = new AtlaspackSourceMap(projectRoot, buffer);
  }

  addVLQMap(
    map: VLQMap,
    lineOffset: number = 0,
    columnOffset: number = 0,
  ): SourceMap {
    let {sourcesContent, sources = [], mappings, names = []} = map;
    if (!sourcesContent) {
      sourcesContent = sources.map(() => '');
    } else {
      sourcesContent = sourcesContent.map((content) =>
        content ? content : '',
      );
    }
    this.sourceMapInstance.addVLQMap(
      mappings,
      sources,
      sourcesContent.map((content) => (content ? content : '')),
      names,
      lineOffset,
      columnOffset,
    );
    return this;
  }

  addSourceMap(sourcemap: SourceMap, lineOffset: number = 0): SourceMap {
    if (!(sourcemap.sourceMapInstance instanceof AtlaspackSourceMap)) {
      throw new Error(
        'The sourcemap provided to addSourceMap is not a valid sourcemap instance',
      );
    }

    this.sourceMapInstance.addSourceMap(
      sourcemap.sourceMapInstance,
      lineOffset,
    );
    return this;
  }

  addBuffer(buffer: Buffer, lineOffset: number = 0): SourceMap {
    let previousMap = new NodeSourceMap(this.projectRoot, buffer);
    return this.addSourceMap(previousMap, lineOffset);
  }

  extends(input: Buffer | SourceMap): SourceMap {
    // $FlowFixMe
    let inputSourceMap: SourceMap = Buffer.isBuffer(input)
      ? new NodeSourceMap(this.projectRoot, input)
      : input;
    this.sourceMapInstance.extends(inputSourceMap.sourceMapInstance);
    return this;
  }

  getNames(): Array<string> {
    return this.sourceMapInstance.getNames();
  }

  getSources(): Array<string> {
    return this.sourceMapInstance.getSources();
  }

  delete() {}

  static generateEmptyMap({
    projectRoot,
    sourceName,
    sourceContent,
    lineOffset = 0,
  }: GenerateEmptyMapOptions): NodeSourceMap {
    let map = new NodeSourceMap(projectRoot);
    map.addEmptyMap(sourceName, sourceContent, lineOffset);
    return map;
  }

  // This function exists to ensure that source map instances from (for example) @parcel/source-map
  // are not used in place of @atlaspack/source-map, as from a JS point of view this is fine, but the
  // underlying buffer may be different, and can cause build time errors.
  static safeToBuffer<T extends SourceMap>(
    sourceMap: T | null | undefined,
  ): Buffer | undefined {
    if (sourceMap == null || sourceMap == undefined) {
      return undefined;
    }

    // We can't use instanceof here because if we're using a resolution for @parcel/source-map,
    // it will be a different instance as it'll be a "copy" of @atlaspack/source-map
    //
    // We only compare the major version number for Atlaspack source maps, so we can take into account linking newer
    // releases into existing codebases.
    if (
      sourceMap.libraryVersion.startsWith(
        SOURCE_MAP_VERSION.substring(0, SOURCE_MAP_VERSION.indexOf('.')),
      )
    ) {
      return sourceMap.toBuffer();
    }

    throw new Error(
      'Source map is not an Atlaspack SourceMap (Expected version ' +
        SOURCE_MAP_VERSION +
        ', got ' +
        sourceMap.libraryVersion +
        ')',
    );
  }
}

export const init: Promise<void> = Promise.resolve();

export type {SourceMap};
