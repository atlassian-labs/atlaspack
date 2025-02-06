const {name: foo} = require('./foo.js');

import('./bar.js').then(({name: bar}) => {
  // eslint-disable-next-line no-undef
  document.getElementById('output').innerText = `Hello, ${foo()} ${bar()}!`;
});
