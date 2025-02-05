Promise.all([import('./foo.js'), import('./bar.js')]).then(
  ([{name: foo}, {name: bar}]) => {
    // eslint-disable-next-line no-undef
    document.getElementById('output').innerText = `Hello, ${foo()} ${bar()}!`;
  },
);
