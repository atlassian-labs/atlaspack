/**
 * @jsxRuntime classic
 * @jsx jsx
 */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("react");
var _reactDefault = parcelHelpers.interopDefault(_react);
var _react1 = require("@compiled/react");
var _css = require("@atlaskit/css");
var listStyles = (0, _css.cssMap)({
    root: {
        alignItems: 'center',
        gap: '4px',
        display: 'flex'
    },
    popupContainer: {
        padding: '8px'
    }
});
function Component(param) {
    var children = param.children;
    return /*#__PURE__*/ (0, _react1.jsx)("div", {
        xcss: (0, _react1.cx)(listStyles.root, listStyles.popupContainer),
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/cx_with_cssmap/in.jsx",
            lineNumber: 22,
            columnNumber: 3
        },
        __self: this
    }, children);
}
