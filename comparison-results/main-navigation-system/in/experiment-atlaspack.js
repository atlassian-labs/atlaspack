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
var contentHeightWhenFixed = "calc(100vh - var(--n_bnrM, 0px) - var(--n_tNvM, 0px))";
var contentInsetBlockStart = "calc(var(--n_bnrM, 0px) + var(--n_tNvM, 0px))";
var mainElementStyles = (0, _react1.cssMap)({
    root: {
        gridArea: 'main',
        isolation: 'isolate',
        insetBlockStart: contentInsetBlockStart,
        overflow: 'auto',
        '@media (min-width: 64rem)': {
            isolation: 'auto',
            height: contentHeightWhenFixed,
            position: 'sticky'
        }
    },
    containPaint: {
        contain: 'paint'
    }
});
function Main(param) {
    var children = param.children, testId = param.testId, id = param.id;
    return /*#__PURE__*/ (0, _react1.jsx)((0, _react.Fragment), {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/main-navigation-system/in.jsx",
            lineNumber: 31,
            columnNumber: 3
        },
        __self: this
    }, /*#__PURE__*/ (0, _react1.jsx)("div", {
        id: id,
        "data-layout-slot": true,
        role: "main",
        css: mainElementStyles.root,
        "data-testid": testId,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/main-navigation-system/in.jsx",
            lineNumber: 32,
            columnNumber: 4
        },
        __self: this
    }, children));
}
