/**
 * @jsxRuntime classic
 * @jsx jsx
 */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Aside", function() {
    return Aside;
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
    },
    inner: {
        insetBlockStart: '48px',
        overflow: 'auto',
        height: '100%',
        '@media (min-width: 64rem)': {
            height: 'calc(100vh - 48px)',
            position: 'sticky'
        }
    }
});
function Aside(param) {
    var children = param.children, xcss = param.xcss, _param_label = param.label, label = _param_label === void 0 ? 'Aside' : _param_label, testId = param.testId;
    return /*#__PURE__*/ (0, _react.jsx)("aside", {
        "aria-label": label,
        css: styles.root,
        "data-testid": testId,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/aside-unused-xcss-prop/in.jsx",
            lineNumber: 35,
            columnNumber: 3
        },
        __self: this
    }, /*#__PURE__*/ (0, _react.jsx)("div", {
        css: styles.inner,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/aside-unused-xcss-prop/in.jsx",
            lineNumber: 40,
            columnNumber: 4
        },
        __self: this
    }, children));
}
