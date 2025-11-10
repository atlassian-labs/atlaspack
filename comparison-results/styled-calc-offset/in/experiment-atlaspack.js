var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _layoutOffset = require("./layout-offset");
var _this = undefined;
var Container = (0, _react.styled).div({
    height: "calc(100vh - ".concat((0, _layoutOffset.LAYOUT_OFFSET), ")")
});
var Component = function() {
    return /*#__PURE__*/ React.createElement(Container, {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-calc-offset/in.jsx",
            lineNumber: 8,
            columnNumber: 32
        },
        __self: _this
    }, "Content");
};
