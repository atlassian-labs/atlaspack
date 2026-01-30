import assert from 'assert';
import NodeSourceMap from '../src/node';
import {SOURCE_MAP_VERSION} from '../src/SourceMap';

describe('NodeSourceMap', () => {
  describe('safeToBuffer', () => {
    it('should return undefined if the source map is null or undefined', () => {
      assert(NodeSourceMap.safeToBuffer(null) === undefined);
      assert(NodeSourceMap.safeToBuffer(undefined) === undefined);
    });

    it('should throw if the version does not have atlaspack: prefix', () => {
      try {
        NodeSourceMap.safeToBuffer({
          libraryVersion: '2.1.1',
          toBuffer: () => Buffer.from('mock'),
        } as any);
        assert.fail('Expected an error to be thrown');
      } catch (e) {
        if (e instanceof Error) {
          assert(
            e.message.includes('Source map is not an Atlaspack SourceMap'),
            'Message was: ' + e.message,
          );
        } else {
          assert.fail('Expected an error to be thrown');
        }
      }
    });

    it('should throw if the major version does not match', () => {
      try {
        NodeSourceMap.safeToBuffer({
          libraryVersion: 'atlaspack:2.1.1',
          toBuffer: () => Buffer.from('mock'),
        } as any);
        assert.fail('Expected an error to be thrown');
      } catch (e) {
        if (e instanceof Error) {
          assert(
            e.message.includes('Source map is not an Atlaspack SourceMap'),
            'Message was: ' + e.message,
          );
        } else {
          assert.fail('Expected an error to be thrown');
        }
      }
    });

    it('should accept source maps with same version', () => {
      const mockBuffer = Buffer.from('mock');
      const result = NodeSourceMap.safeToBuffer({
        libraryVersion: SOURCE_MAP_VERSION,
        toBuffer: () => mockBuffer,
      } as any);
      assert.strictEqual(result, mockBuffer);
    });

    it('should accept source maps with same major version but different minor/patch', () => {
      const mockBuffer = Buffer.from('mock');
      const result = NodeSourceMap.safeToBuffer({
        libraryVersion: SOURCE_MAP_VERSION.substring(
          0,
          SOURCE_MAP_VERSION.indexOf('.'),
        ),
        toBuffer: () => mockBuffer,
      } as any);
      assert.strictEqual(result, mockBuffer);
    });
  });
});
