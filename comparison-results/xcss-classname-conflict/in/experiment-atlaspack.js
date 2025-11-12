/**
 * @jsxRuntime classic
 * @jsx jsx
 * @jsxFrag
 */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Main", function() {
    return Main;
});
var _react = require("react");
var _react1 = require("@compiled/react");
var mainElementStyles = (0, _react1.cssMap)({
    root: {
        gridArea: 'main',
        isolation: 'isolate',
        insetBlockStart: '48px',
        overflow: 'auto',
        '@media (min-width: 64rem)': {
            isolation: 'auto',
            height: 'calc(100vh - 48px)',
            position: 'sticky'
        }
    }
});
function Main(param) {
    var children = param.children, xcss = param.xcss, testId = param.testId;
    return /*#__PURE__*/ (0, _react1.jsx)((0, _react.Fragment), {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/xcss-classname-conflict/in.jsx",
            lineNumber: 30,
            columnNumber: 3
        },
        __self: this
    }, /*#__PURE__*/ (0, _react1.jsx)("div", {
        className: xcss,
        role: "main",
        css: mainElementStyles.root,
        "data-testid": testId,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/xcss-classname-conflict/in.jsx",
            lineNumber: 31,
            columnNumber: 4
        },
        __self: this
    }, children));
}
