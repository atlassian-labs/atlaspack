import {a, b, c} from './sync.js';
import './sync2.js';

import('./async');
import('./async2');

const x = () => console.log(a, b, c);

class Test {}
new Test();

x();