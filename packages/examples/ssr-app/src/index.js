const {renderToString} = require('react-dom/server');
const App = require('./App');

const init = async () => {
  console.log('SSR app initialized');
  return 'initialized';
};

const _default = async (input) => {
  console.log('Rendering with input:', input);

  const html = renderToString(<App input={input} />);

  return {
    html,
    input,
  };
};

module.exports = {};
module.exports.init = init;
module.exports.default = _default;
