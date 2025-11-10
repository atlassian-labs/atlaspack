/**
 * @jsxRuntime classic
 * @jsx jsx
 */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Root", function() {
    return Root;
});
var _react = require("react");
var _reactDefault = parcelHelpers.interopDefault(_react);
var _react1 = require("@compiled/react");
var styles = (0, _react1.cssMap)({
    root: {
        display: 'grid',
        minHeight: '100vh',
        gridTemplateAreas: '\n            "banner"\n            "top-bar"\n            "main"\n            "aside"\n       ',
        gridTemplateColumns: 'minmax(0, 1fr)',
        gridTemplateRows: 'auto auto 1fr auto',
        '@media (min-width: 64rem)': {
            gridTemplateAreas: '\n            "banner banner banner"\n            "top-bar top-bar top-bar"\n            "side-nav main aside"\n       ',
            gridTemplateRows: 'auto auto 3fr',
            gridTemplateColumns: 'auto minmax(0,1fr) auto'
        },
        '> :not([data-layout-slot])': {
            display: 'none !important'
        }
    }
});
function Root(param) {
    var children = param.children, xcss = param.xcss, testId = param.testId;
    return /*#__PURE__*/ (0, _react1.jsx)("div", {
        css: styles.root,
        className: xcss,
        "data-testid": testId,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/navigation-system-root-xcss-classname/in.jsx",
            lineNumber: 37,
            columnNumber: 3
        },
        __self: this
    }, children);
}
