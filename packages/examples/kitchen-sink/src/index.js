import atlaspack from 'url:./atlaspack.webp';
import {message} from './message';

import('./async');
import('./async2');

new Worker(new URL('worker.js', import.meta.url), {type: 'module'});

console.log(message);

let icon = document.createElement('img');
icon.src = atlaspack;
icon.width = 100;

document.body.prepend(icon);
