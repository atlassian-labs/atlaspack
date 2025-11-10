/**
 * @jsxRuntime classic
 * @jsx jsx
 */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Root", function() {
    return Root;
});
var _react = require("@compiled/react");
var styles = (0, _react.cssMap)({
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
        }
    }
});
function Root(param) {
    var children = param.children, xcss = param.xcss, testId = param.testId;
    return /*#__PURE__*/ (0, _react.jsx)("div", {
        css: styles.root,
        className: xcss,
        "data-testid": testId,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/xcss-prop-as-classname/in.jsx",
            lineNumber: 33,
            columnNumber: 3
        },
        __self: this
    }, children);
}
