import assert from 'assert';
import NodeSourceMap from '../src/node';

describe('NodeSourceMap', () => {
  describe('safeToBuffer', () => {
    it('should return undefined if the source map is null or undefined', () => {
      assert(NodeSourceMap.safeToBuffer(null) === undefined);
      assert(NodeSourceMap.safeToBuffer(undefined) === undefined);
    });

    it('should throw if the version does not match', () => {
      try {
        NodeSourceMap.safeToBuffer({
          libraryVersion: 'different-version',
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
  });
});
