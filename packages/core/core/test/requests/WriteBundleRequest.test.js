// @flow strict-local

import {replaceStream} from '../../src/requests/WriteBundleRequest';
import {Writable, pipeline} from 'stream';
import {replaceHashReferences} from '@atlaspack/rust';
import assert from 'assert';
import {blobToStream} from '@atlaspack/utils';

class BufferWritable extends Writable {
  buffers: Buffer[];

  constructor() {
    super();
    this.buffers = [];
  }

  getBuffer(): Buffer {
    return Buffer.concat(this.buffers);
  }

  _write(chunk: string | Buffer, encoding: string, callback) {
    let buffer =
      typeof chunk === 'string'
        ? // $FlowFixMe
          Buffer.from(chunk, encoding)
        : chunk;
    this.buffers.push(buffer);
    callback();
  }
}

function generateSampleInput(
  hashReferences: Map<string, string>,
  inputSize: number,
): Buffer {
  let output = '';
  const hashReferenceKeys = Array.from(hashReferences.keys());

  while (output.length < inputSize) {
    if (Math.random() < 0.1) {
      const hash =
        hashReferenceKeys[Math.floor(Math.random() * hashReferenceKeys.length)];
      output += hash;
    } else {
      for (let i = 0; i < 16; i++) {
        output += String.fromCharCode(
          'a'.charCodeAt(0) + Math.floor(Math.random() * 26),
        );
      }
    }
  }

  return Buffer.from(output);
}

async function javascriptReplaceHashReferences(
  buffer: Buffer,
  hashReferences: Map<string, string>,
): Promise<Buffer> {
  const writable = new BufferWritable();
  const transform = replaceStream(hashReferences);

  await new Promise((resolve, reject) => {
    pipeline(blobToStream(buffer), transform, writable, (err) => {
      if (err) reject(err);
      else resolve(null);
    });
  });

  const output = writable.getBuffer();
  return output;
}

describe('replaceStream', () => {
  const hashReferences = new Map([
    ['HASH_REF_1234567890123456', 'HASH_REF_replacedwithstri'],
  ]);
  const leftPad = (s: string, length: number) => {
    let output = '';
    const pad = length - s.length;
    for (let i = 0; i < pad; i++) {
      output += '0';
    }
    output += s;
    return output;
  };

  for (let i = 0; i < 500; i++) {
    const key = `HASH_REF_${leftPad(String(i), 16)}`;
    const replacement = 'HASH_REF_replacedwithstri';
    assert.equal(key.length, replacement.length);
    hashReferences.set(key, replacement);
  }

  it('leftpad', () => {
    assert.equal(leftPad('s', 5), '0000s');
  });

  const buffer = Buffer.from(
    'Hello HASH_REF_1234567890123456 more stuff here HASH_REF_1234567890123456',
  );
  const expectedOutput =
    'Hello HASH_REF_replacedwithstri more stuff here HASH_REF_replacedwithstri';

  it('javascript - replaces hash references in buffers', async () => {
    const output = await javascriptReplaceHashReferences(
      buffer,
      hashReferences,
    );
    const outputString = output.toString();
    assert.equal(outputString, expectedOutput);
  });

  it('rust - replaces hash references in buffers', () => {
    const output = replaceHashReferences(
      buffer,
      Object.fromEntries(hashReferences.entries()),
    );

    const outputString = output.toString();
    assert.equal(outputString, expectedOutput);
  });

  let currentInputSize = 1024 * 1024; // Start at 1MB and grow to 2MB
  const maximumInputSize = 2 * 1024 * 1024;
  const inputSizes = [];
  while (currentInputSize <= maximumInputSize) {
    inputSizes.push(currentInputSize);
    currentInputSize *= 2;
  }

  inputSizes.forEach((inputSize) => {
    describe('input size ' + inputSize / 1024 / 1024 + 'MB', () => {
      const buffer = generateSampleInput(hashReferences, inputSize);
      let expectedOutput = buffer.toString();
      for (let [key, replacement] of hashReferences.entries()) {
        expectedOutput = expectedOutput.replaceAll(key, replacement);
      }

      it('javascript - works on huge buffer', async () => {
        const output = await javascriptReplaceHashReferences(
          buffer,
          hashReferences,
        );
        const outputString = output.toString();
        assert.equal(outputString, expectedOutput);
      });

      it('rust - works on huge buffer', () => {
        const output = replaceHashReferences(
          buffer,
          Object.fromEntries(hashReferences.entries()),
        );

        const outputString = output.toString();
        assert.equal(outputString, expectedOutput);
      });
    });
  });
});
