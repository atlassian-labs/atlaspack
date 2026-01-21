// Polyfill TextEncoder/TextDecoder (for Tesseract runtime)
if (typeof TextEncoder === 'undefined') {
  globalThis.TextEncoder = class TextEncoder {
    encode(str) {
      const utf8 = unescape(encodeURIComponent(str));
      const bytes = new Uint8Array(utf8.length);
      for (let i = 0; i < utf8.length; i++) {
        bytes[i] = utf8.charCodeAt(i);
      }
      return bytes;
    }
  };
}

if (typeof TextDecoder === 'undefined') {
  globalThis.TextDecoder = class TextDecoder {
    decode(bytes) {
      const utf8 = Array.from(bytes, (byte) => String.fromCharCode(byte)).join(
        '',
      );
      return decodeURIComponent(escape(utf8));
    }
  };
}

const React = require('react');
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
