/*
 * BitSets are primarily used for performance reasons, so we want to be able to
 * validate that changes we make to it are performace improvements.
 *
 * This file exists as a ready-made playground for benchmarking changes you may
 * want to make to the BitSet implementation.
 */

// Needed to make Flow work in the benchmarks
require('@atlaspack/babel-register');

const {BitSet} = require('../src/BitSet.js');
const b = require('benny');

function createBitSetWithEntries(capacity, entries) {
  let bitSet = new BitSet(capacity);
  for (const index of entries) {
    bitSet.add(index);
  }
  return bitSet;
}

let bundleIndices = [4334, 348, 2145, 480, 747, 1446, 326, 2791, 2658, 1334];

let bundleBitSet = createBitSetWithEntries(5000, bundleIndices);

b.suite(
  'BitSet - size',
  b.add('Control', () => {
    bundleBitSet.size();
  }),
  b.configure({minSamples: 100}),
  b.cycle(),
  b.complete(),
);
