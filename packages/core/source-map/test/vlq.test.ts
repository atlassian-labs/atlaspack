import assert from 'assert';
import {decodeVLQ, encodeVLQ} from '../src/vlq';

// Reference table of (value, expected VLQ string) pairs.
// Single-group values (abs <= 15) can be verified by hand:
//   encodeVLQ(n) = BASE64_CHARS[(n << 1) | sign]  (sign=1 if negative)
// Multi-group values are verified via round-trip in the round-trip suite.
const CASES: Array<[number, string]> = [
  [0, 'A'],
  [1, 'C'],
  [-1, 'D'],
  [2, 'E'],
  [-2, 'F'],
  [15, 'e'],
  [-15, 'f'],
  // Values that require two base64 groups (abs >= 16)
  [16, 'gB'],
  [-16, 'hB'],
  [1000, 'w+B'],
  [-1000, 'x+B'],
  // Larger values (four groups)
  [100000, 'gqjG'],
  [-100000, 'hqjG'],
];

describe('VLQ codec', () => {
  describe('encodeVLQ', () => {
    for (const [value, expected] of CASES) {
      it(`encodes ${value} as "${expected}"`, () => {
        assert.strictEqual(encodeVLQ(value), expected);
      });
    }

    it('encodes zero', () => {
      assert.strictEqual(encodeVLQ(0), 'A');
    });

    it('encodes large positive value', () => {
      // 2^20 = 1048576
      const encoded = encodeVLQ(1048576);
      assert.ok(encoded.length > 0);
      // Round-trip must recover the original value
      assert.strictEqual(decodeVLQ(encoded, 0).value, 1048576);
    });

    it('encodes large negative value', () => {
      const encoded = encodeVLQ(-1048576);
      assert.strictEqual(decodeVLQ(encoded, 0).value, -1048576);
    });
  });

  describe('decodeVLQ', () => {
    for (const [expected, encoded] of CASES) {
      it(`decodes "${encoded}" as ${expected}`, () => {
        const {value, nextPos} = decodeVLQ(encoded, 0);
        assert.strictEqual(value, expected);
        assert.strictEqual(nextPos, encoded.length);
      });
    }

    it('returns correct nextPos when decoding from mid-string', () => {
      // Encode two values back-to-back and decode the second one starting
      // after the first.
      const a = encodeVLQ(42);
      const b = encodeVLQ(-7);
      const combined = a + b;
      const {value, nextPos} = decodeVLQ(combined, a.length);
      assert.strictEqual(value, -7);
      assert.strictEqual(nextPos, combined.length);
    });

    it('decodes a multi-value sequence positionally', () => {
      const values = [0, 1, -1, 100, -100, 0];
      const encoded = values.map(encodeVLQ).join('');
      let pos = 0;
      for (const expected of values) {
        const {value, nextPos} = decodeVLQ(encoded, pos);
        assert.strictEqual(value, expected);
        pos = nextPos;
      }
      assert.strictEqual(pos, encoded.length);
    });
  });

  describe('round-trip', () => {
    const roundTripValues = [
      0, 1, -1, 2, -2, 15, -15, 16, -16, 31, -31, 32, -32, 127, -127, 128, -128,
      255, -255, 1000, -1000, 65535, -65535, 1048575, -1048575,
    ];

    for (const value of roundTripValues) {
      it(`round-trips ${value}`, () => {
        const encoded = encodeVLQ(value);
        const {value: decoded, nextPos} = decodeVLQ(encoded, 0);
        assert.strictEqual(decoded, value);
        assert.strictEqual(nextPos, encoded.length);
      });
    }
  });
});
