var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _tokens = require("@atlaskit/tokens");
var _this = undefined;
var Box = (0, _react.styled).div({
    backgroundImage: "linear-gradient(\n    to right,\n    ".concat((0, _tokens.token)('color.background.neutral'), " 10%,\n    ").concat((0, _tokens.token)('color.background.neutral.subtle'), " 30%,\n    ").concat((0, _tokens.token)('color.background.neutral'), " 50%\n  )")
});
var Component = function() {
    return /*#__PURE__*/ React.createElement(Box, {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/tokens-gradient/in.jsx",
            lineNumber: 13,
            columnNumber: 32
        },
        __self: _this
    });
};
