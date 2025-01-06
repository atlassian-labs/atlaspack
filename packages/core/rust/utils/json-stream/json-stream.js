/**
 * @description This will parse a single buffer of multiple
 * JSON documents.
 */
class JsonStream {
  /**
   * @description Will parse a list of JSON documents, each
   * document should start with the length of the document's
   * bytes formatted as a u32 across 8 bytes in Little Endian format
   *
   * Buffer.from([...dataLengthLE, ...data])
   */
  static *parseLE(/** @type {Buffer} */ data) {
    let cursor = 0;

    while (cursor < data.length) {
      const headerEnd = cursor + 8;
      const headerBytes = data.subarray(cursor, headerEnd);

      const dataLength = headerBytes.readUint32LE();
      const dataEnd = headerEnd + dataLength;

      const bodyBytes = data.subarray(headerEnd, dataEnd);
      yield JSON.parse(bodyBytes);

      cursor = dataEnd;
    }
  }
}

module.exports.JsonStream = JsonStream;
