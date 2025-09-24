// Fake React module that mimics the export pattern
exports.Fragment = 0xeacb;
exports.StrictMode = 0xeacc;
exports.createElement = function(type, props, ...children) {
  return { type, props, children };
};
exports.useState = function(initial) {
  return [initial, function() {}];
};
