import { one } from './one';

console.log('two.js executing, one is:', one);

const two = 'two';

exports.two = two;
exports.both = [one, two];