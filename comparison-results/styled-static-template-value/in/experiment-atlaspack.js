var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _this = undefined;
var GRID = 8;
var SIZE = GRID * 2;
var Icon = (0, _react.styled).div({
    width: "".concat(SIZE, "px"),
    minWidth: "".concat(SIZE, "px"),
    height: "".concat(SIZE, "px"),
    flexBasis: "".concat(SIZE, "px")
});
var Component = function() {
    return /*#__PURE__*/ React.createElement(Icon, {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-static-template-value/in.jsx",
            lineNumber: 13,
            columnNumber: 32
        },
        __self: _this
    });
};
