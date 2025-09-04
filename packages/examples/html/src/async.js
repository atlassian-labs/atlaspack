console.log(require('react'));
require('lodash');
import './child.css';
console.log('async');

import {a, b, c, d} from './sync.js';

console.log(a, b, c, d);

export class Foo {}
new Foo();
