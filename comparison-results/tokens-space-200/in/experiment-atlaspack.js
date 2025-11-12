var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _this = undefined;
var styles = (0, _react.cssMap)({
    button: {
        paddingInline: 'var(--ds-space-200, 16px)'
    }
});
var Component = function() {
    return /*#__PURE__*/ React.createElement("div", {
        className: styles.button(),
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/tokens-space-200/in.jsx",
            lineNumber: 9,
            columnNumber: 32
        },
        __self: _this
    }, "Content");
};
