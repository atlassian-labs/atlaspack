require('@atlaspack/babel-register');
const b = require('benny');

const {setIntersect, setIntersectStatic} = require('./src/collection.js');

const setA = new Set([
  23, 25, 29, 29, 12, 16, 14, 23, 18, 19, 16, 24, 9, 29, 26,
]);
const setB = new Set([24, 1, 3, 6, 1, 3, 1, 5, 20, 15, 21, 23, 13, 16, 6]);

b.suite(
  'Collection - set intersection',
  b.add('Control', () => {
    const setClone = new Set(setA);
    return () => setIntersect(setClone, setB);
  }),
  b.add('setIntersectStatic', () => {
    setIntersectStatic(setA, setB);
  }),
  b.configure({minSamples: 100}),
  b.cycle(),
  b.complete(),
);
