var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _this = undefined;
var styles = (0, _react.cssMap)({
    primary: {
        color: 'salmon'
    },
    secondary: {
        color: 'goldenrod'
    }
});
var Component = function() {
    return /*#__PURE__*/ React.createElement("div", {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/css-map/in.jsx",
            lineNumber: 13,
            columnNumber: 3
        },
        __self: _this
    }, /*#__PURE__*/ React.createElement("span", {
        className: styles.primary(),
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/css-map/in.jsx",
            lineNumber: 14,
            columnNumber: 5
        },
        __self: _this
    }, "Primary"), /*#__PURE__*/ React.createElement("span", {
        className: styles.secondary(),
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/css-map/in.jsx",
            lineNumber: 15,
            columnNumber: 5
        },
        __self: _this
    }, "Secondary"));
};
