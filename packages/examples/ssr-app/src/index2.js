const {renderToString} = require('react-dom/server');
const App2 = require('./App2');

const init = async () => {
  console.log('SSR app 2 initialized');
  return 'initialized';
};

const _default = async (input) => {
  console.log('Rendering app 2 with input:', input);

  const html = renderToString(<App2 input={input} />);

  return {
    html,
    input,
  };
};

module.exports = {};
module.exports.init = init;
module.exports.default = _default;
