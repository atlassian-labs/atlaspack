var _react = require("@compiled/react");
var styles = (0, _react.cssMap)({
    root: {
        display: 'grid',
        minHeight: '100vh',
        gridTemplateAreas: '\n      "banner"\n      "top-bar"\n      "main"\n      "aside"\n    ',
        gridTemplateColumns: 'minmax(0, 1fr)',
        gridTemplateRows: 'auto auto 1fr auto',
        '@media (min-width: 64rem)': {
            gridTemplateAreas: '\n        "banner banner banner"\n        "top-bar top-bar top-bar"\n        "side-nav main aside"\n      ',
            gridTemplateRows: 'auto auto 3fr',
            gridTemplateColumns: 'auto minmax(0,1fr) auto'
        }
    }
});
function Root(param) {
    var xcss = param.xcss;
    return /*#__PURE__*/ React.createElement("div", {
        css: styles.root,
        className: xcss,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/xcss-className-prop-conflict/in.jsx",
            lineNumber: 29,
            columnNumber: 5
        },
        __self: this
    }, "Content");
}
