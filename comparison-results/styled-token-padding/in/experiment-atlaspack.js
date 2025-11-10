var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("react");
var _reactDefault = parcelHelpers.interopDefault(_react);
var _react1 = require("@compiled/react");
var _tokens = require("@atlaskit/tokens");
var _this = undefined;
var Wrapper = (0, _react1.styled).div({
    padding: "".concat((0, _tokens.token)('space.050'), " ").concat((0, _tokens.token)('space.150'), " ").concat((0, _tokens.token)('space.150'), " ").concat(function(param) {
        var padded = param.padded;
        return padded ? (0, _tokens.token)('space.150') : (0, _tokens.token)('space.0');
    })
});
var Component = function() {
    return /*#__PURE__*/ (0, _reactDefault.default).createElement(Wrapper, {
        padded: true,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-token-padding/in.jsx",
            lineNumber: 10,
            columnNumber: 32
        },
        __self: _this
    });
};
