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
): Buffe {
  let output = '';

  while (output.length < inputSize) {
    if (Math.random() < 0.1) {
      output += 'HASH_REF_1234567890123456';
    } else {
      for (let i = 0; i < 16; i++) {
        output += String.fromCharCode(97 + Math.floor(Math.random() * 26));
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
    pipeline(blobToStream(buffer), transform, writable, err => {
      if (err) reject(err);
      else resolve(null);
    });
  });

  const output = writable.getBuffer();
  return output;
}

describe.only('replaceStream', () => {
  const hashReferences = new Map([
    ['HASH_REF_1234567890123456', 'HASH_REF_replacedwithstri'],
  ]);
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

  let currentInputSize = 1024 * 1024; // Start at 1MB and grow to 40MB
  const inputSizes = [];
  while (currentInputSize < 80 * 1024 * 1024) {
    inputSizes.push(currentInputSize);
    currentInputSize *= 2;
  }

  inputSizes.forEach(inputSize => {
    describe('input size ' + inputSize / 1024 / 1024 + 'MB', () => {
      const buffer = generateSampleInput(hashReferences, inputSize);
      const expectedOutput = buffer
        .toString()
        .replace(/HASH_REF_1234567890123456/g, 'HASH_REF_replacedwithstri');

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
