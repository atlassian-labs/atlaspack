/**
 * @jsxRuntime classic
 * @jsx jsx
 */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var styles = (0, _react.cssMap)({
    root: {
        gridArea: 'aside',
        boxSizing: 'border-box',
        position: 'relative',
        '@media (min-width: 64rem)': {
            width: 'var(--aside-width)',
            justifySelf: 'end'
        }
    }
});
function Component(param) {
    var xcss = param.xcss, children = param.children;
    return /*#__PURE__*/ (0, _react.jsx)("aside", {
        css: styles.root,
        className: xcss,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/css-and-classname-conflict/in.jsx",
            lineNumber: 21,
            columnNumber: 3
        },
        __self: this
    }, children);
}
