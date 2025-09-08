import {a, b, c, d, default as obj, func} from './sync.js';

const runAsync = async () => {
  const async = await import('./async');
  console.log(async.Foo);
};

import('./async2');
const x = () => console.log(a, b, c, d, obj, func);

class Test {}
new Test();

x();
runAsync();
