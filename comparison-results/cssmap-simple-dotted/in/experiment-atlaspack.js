var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _this = undefined;
var styles = (0, _react.cssMap)({
    container: {
        display: 'flex'
    },
    textBold: {
        color: '#292A2E'
    }
});
var Component = function() {
    return /*#__PURE__*/ React.createElement("div", {
        css: styles.container,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/cssmap-simple-dotted/in.jsx",
            lineNumber: 13,
            columnNumber: 3
        },
        __self: _this
    }, /*#__PURE__*/ React.createElement("span", {
        css: styles.textBold,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/cssmap-simple-dotted/in.jsx",
            lineNumber: 14,
            columnNumber: 5
        },
        __self: _this
    }, "Content"));
};
