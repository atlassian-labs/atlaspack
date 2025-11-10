var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _this = undefined;
var pulse = (0, _react.keyframes)({
    '0%': {
        transform: 'scale(1)'
    },
    '50%': {
        transform: 'scale(1.1)'
    },
    '100%': {
        transform: 'scale(1)'
    }
});
var StyledDiv = (0, _react.styled).div({
    animation: "".concat(pulse, " 2s infinite")
});
var Component = function() {
    return /*#__PURE__*/ React.createElement(StyledDiv, {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-and-keyframes/in.jsx",
            lineNumber: 19,
            columnNumber: 32
        },
        __self: _this
    }, "Pulse");
};
