// @flow strict-local

import {Readable} from 'stream';

import {deserializeRaw} from '@atlaspack/build-cache';
import type {AST, Blob} from '@atlaspack/types';
import {bufferStream, blobToStream, streamFromPromise} from '@atlaspack/utils';
import SourceMap from '@parcel/source-map';

import {generateFromAST} from './assetUtils';
import type {Asset, Dependency, AtlaspackOptions} from './types';

export default class CommittedAsset {
  key: ?string;
  value: Asset;
  options: AtlaspackOptions;
  content: ?Promise<Buffer | string>;
  mapBuffer: ?Promise<?Buffer>;
  map: ?Promise<?SourceMap>;
  ast: ?Promise<AST>;
  code: ?string;

  constructor(value: Asset, options: AtlaspackOptions) {
    this.value = value;
    this.key = this.value.contentKey;
    this.options = options;
  }

  getContent(): Blob | Promise<Buffer | string> {
    if (this.content == null) {
      if (this.key != null) {
        if (this.value.isLargeBlob) {
          return this.options.cache.getStream(this.key);
        } else {
          return this.options.cache.getBlob(this.key);
        }
      } else if (this.value.astKey != null) {
        return streamFromPromise(
          generateFromAST(this).then(({content}) => {
            if (!(content instanceof Readable)) {
              this.content = Promise.resolve(content);
            }
            return content;
          }),
        );
      } else {
        throw new Error('Asset has no content');
      }
    }

    return this.content;
  }

  async getCode(): Promise<string> {
    let content;
    if (this.content == null && this.key != null) {
      this.content = this.options.cache.getBlob(this.key);
      content = await this.content;
    } else {
      content = await this.getContent();
    }

    if (typeof content === 'string' || content instanceof Buffer) {
      return content.toString();
    } else if (content != null) {
      this.content = bufferStream(content);
      return (await this.content).toString();
    }

    return '';
  }

  async getBuffer(): Promise<Buffer> {
    let content = await this.getContent();

    if (content == null) {
      return Buffer.alloc(0);
    } else if (typeof content === 'string' || content instanceof Buffer) {
      return Buffer.from(content);
    }

    this.content = bufferStream(content);
    return this.content;
  }

  getStream(): Readable {
    let content = this.getContent();
    return content instanceof Promise
      ? streamFromPromise(content)
      : blobToStream(content);
  }

  getMapBuffer(): Promise<?Buffer> {
    let mapKey = this.value.mapKey;
    if (mapKey != null && this.mapBuffer == null) {
      this.mapBuffer = (async () => {
        try {
          // Handle v3 assets that were processed by the native asset graph code (i.e. not runtime
          // or helper assets)
          if (
            this.options.featureFlags.atlaspackV3 &&
            this.value.meta.isV3 === true
          ) {
            let buffer = await this.options.cache.getBuffer(mapKey);
            if (!buffer) {
              return buffer;
            }

            // We make the conversion from json to buffer here, since both js packagers lazily
            // instantiate a sourcemap from the map buffer. This ensures we do not have to modify
            // the packagers in any significant way. Additionally, storing the SourceMap objects
            // upfront in these packagers appears to make larger builds more prone to failing.
            let sourceMap = new SourceMap(this.options.projectRoot);
            sourceMap.addVLQMap(JSON.parse(buffer.toString()));
            return sourceMap.toBuffer();
          }

          return await this.options.cache.getBlob(mapKey);
        } catch (err) {
          if (err.code === 'ENOENT' && this.value.astKey != null) {
            return (await generateFromAST(this)).map?.toBuffer();
          } else {
            throw err;
          }
        }
      })();
    }

    return this.mapBuffer ?? Promise.resolve();
  }

  getMap(): Promise<?SourceMap> {
    if (this.map == null) {
      this.map = (async () => {
        let mapBuffer = await this.getMapBuffer();
        if (mapBuffer) {
          // Get sourcemap from flatbuffer
          return new SourceMap(this.options.projectRoot, mapBuffer);
        }
      })();
    }

    return this.map;
  }

  getAST(): Promise<?AST> {
    if (this.value.astKey == null) {
      return Promise.resolve(null);
    }

    if (this.ast == null) {
      this.ast = this.options.cache
        .getBlob(this.value.astKey)
        .then((serializedAst) => deserializeRaw(serializedAst));
    }

    return this.ast;
  }

  getDependencies(): Array<Dependency> {
    return Array.from(this.value.dependencies.values());
  }
}
