var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _this = undefined;
var styles = (0, _react.cssMap)({
    container: {
        display: 'flex',
        padding: '8px'
    },
    text: {
        color: 'red',
        fontSize: '14px'
    }
});
var Component = function() {
    return /*#__PURE__*/ React.createElement("div", {
        css: styles.container,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/simple-cssmap-unbounded/in.jsx",
            lineNumber: 15,
            columnNumber: 3
        },
        __self: _this
    }, /*#__PURE__*/ React.createElement("span", {
        css: styles.text,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/simple-cssmap-unbounded/in.jsx",
            lineNumber: 16,
            columnNumber: 5
        },
        __self: _this
    }, "Hello"));
};
