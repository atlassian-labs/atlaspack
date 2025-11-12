var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _this = undefined;
var styles = (0, _react.css)({
    color: 'red',
    backgroundColor: 'transparent'
});
var Component = function() {
    return /*#__PURE__*/ React.createElement("div", {
        css: styles,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/basic-css/in.jsx",
            lineNumber: 6,
            columnNumber: 32
        },
        __self: _this
    }, "Hello");
};
