import {a, b, c, d} from './sync.js';

import('./async');
import('./async2');

const x = () => console.log(a, b, c, d);

class Test {}
new Test();

x();