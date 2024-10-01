import type {AST, Blob} from '@atlaspack/types';
import type {Asset, Dependency, AtlaspackOptions} from './types';

import {Readable} from 'stream';
import SourceMap from '@parcel/source-map';
import {bufferStream, blobToStream, streamFromPromise} from '@atlaspack/utils';
import {generateFromAST} from './assetUtils';
import {deserializeRaw} from './serializer';

export default class CommittedAsset {
  key: string | null | undefined;
  value: Asset;
  options: AtlaspackOptions;
  content: Promise<Buffer | string> | null | undefined;
  mapBuffer: Promise<Buffer | null | undefined> | null | undefined;
  map: Promise<SourceMap | null | undefined> | null | undefined;
  ast: Promise<AST> | null | undefined;
  code: string | null | undefined;
  generatingPromise: Promise<undefined> | null | undefined;

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
    // @ts-expect-error - TS2322 - Type 'string | Buffer' is not assignable to type 'Buffer'.
    return this.content;
  }

  getStream(): Readable {
    let content = this.getContent();
    return content instanceof Promise
      ? streamFromPromise(content)
      : blobToStream(content);
  }

  getMapBuffer(): Promise<Buffer | null | undefined> {
    let mapKey = this.value.mapKey;
    if (mapKey != null && this.mapBuffer == null) {
      this.mapBuffer = (async () => {
        try {
          return await this.options.cache.getBlob(mapKey);
        } catch (err: any) {
          if (err.code === 'ENOENT' && this.value.astKey != null) {
            return (await generateFromAST(this)).map?.toBuffer();
          } else {
            throw err;
          }
        }
      })();
    }

    // @ts-expect-error - TS2322 - Type 'Promise<void> | Promise<Buffer | null | undefined>' is not assignable to type 'Promise<Buffer | null | undefined>'.
    return this.mapBuffer ?? Promise.resolve();
  }

  getMap(): Promise<SourceMap | null | undefined> {
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

  getAST(): Promise<AST | null | undefined> {
    if (this.value.astKey == null) {
      return Promise.resolve(null);
    }

    if (this.ast == null) {
      this.ast = this.options.cache
        .getBlob(this.value.astKey)
        // @ts-expect-error - TS7006 - Parameter 'serializedAst' implicitly has an 'any' type.
        .then((serializedAst) => deserializeRaw(serializedAst));
    }

    // @ts-expect-error - TS2322 - Type 'Promise<AST> | null | undefined' is not assignable to type 'Promise<AST | null | undefined>'.
    return this.ast;
  }

  getDependencies(): Array<Dependency> {
    return Array.from(this.value.dependencies.values());
  }
}
