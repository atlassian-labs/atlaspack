import('./foo.js').then(({name}) => {
  // eslint-disable-next-line no-undef
  document.getElementById('output').innerText = `Hello, ${name()}!`;
});
