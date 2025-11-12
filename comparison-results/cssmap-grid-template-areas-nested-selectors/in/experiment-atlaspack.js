/**
 * @jsxRuntime classic
 * @jsx jsx
 */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "GridLayout", function() {
    return GridLayout;
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
        '@media (min-width: 90rem)': {
            gridTemplateAreas: '\n                "banner banner banner banner"\n                "top-bar top-bar top-bar top-bar"\n                "side-nav main aside panel"\n           ',
            gridTemplateRows: 'auto auto 3fr',
            gridTemplateColumns: 'auto minmax(0,1fr) auto auto'
        },
        '> :not([data-layout-slot])': {
            display: 'none !important'
        }
    }
});
function GridLayout(param) {
    var children = param.children;
    return /*#__PURE__*/ (0, _react1.jsx)("div", {
        css: styles.root,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/cssmap-grid-template-areas-nested-selectors/in.jsx",
            lineNumber: 46,
            columnNumber: 3
        },
        __self: this
    }, children);
}
