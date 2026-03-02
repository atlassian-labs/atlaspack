/**
 * Pure-JS VLQ codec for source map mappings strings.
 *
 * Source map VLQ encoding:
 *   - Each value is encoded as one or more 6-bit base64 groups.
 *   - Bit 5 of each group is the continuation flag; bits 4–0 are data.
 *   - In the very first group, bit 0 is the sign bit and bits 4–1 are value
 *     bits (so the effective data width of the first group is 4 bits).
 *   - All subsequent groups contribute 5 data bits each.
 *
 * Lookup tables are allocated once at module load time.
 */

const BASE64_CHARS =
  'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

// Maps char-code → base64 digit value (−1 for invalid characters).
const BASE64_VALUES = new Int8Array(128).fill(-1);
for (let i = 0; i < BASE64_CHARS.length; i++) {
  BASE64_VALUES[BASE64_CHARS.charCodeAt(i)] = i;
}

export type VLQDecodeResult = {value: number; nextPos: number};

/**
 * Decodes a single VLQ-encoded integer from `str` starting at `pos`.
 * Returns the decoded value and the position of the first character after the
 * encoded value (i.e. the start of the next field or segment delimiter).
 */
export function decodeVLQ(str: string, pos: number): VLQDecodeResult {
  let result = 0;
  let shift = 0;
  let continuation: boolean;
  do {
    const digit = BASE64_VALUES[str.charCodeAt(pos++)];
    continuation = (digit & 0x20) !== 0;
    result |= (digit & 0x1f) << shift;
    shift += 5;
  } while (continuation);
  const negate = (result & 1) !== 0;
  result >>>= 1;
  return {value: negate ? -result : result, nextPos: pos};
}

/**
 * Encodes a single integer as a VLQ string.
 */
export function encodeVLQ(value: number): string {
  let vlq = value < 0 ? (-value << 1) | 1 : value << 1;
  let result = '';
  do {
    let digit = vlq & 0x1f;
    vlq >>>= 5;
    if (vlq > 0) digit |= 0x20;
    result += BASE64_CHARS[digit];
  } while (vlq > 0);
  return result;
}
