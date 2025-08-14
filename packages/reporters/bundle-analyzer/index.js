'use strict';

Object.defineProperty(exports, '__esModule', {
  value: true,
});
var _BundleAnalyzerReporter = require('./src/BundleAnalyzerReporter');

Object.keys(_BundleAnalyzerReporter).forEach(function (key) {
  if (key === 'default' || key === '__esModule') return;
  if (key in exports && exports[key] === _BundleAnalyzerReporter[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _BundleAnalyzerReporter[key];
    },
  });
});
