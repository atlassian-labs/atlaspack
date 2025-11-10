var _react = require("@compiled/react");
var asideVar = '--aside-var';
var panelSplitterResizingVar = '--n_asdRsz';
var contentInsetBlockStart = 'var(--content-inset-block-start)';
var contentHeightWhenFixed = 'var(--content-height-when-fixed)';
var styles = (0, _react.cssMap)({
    root: {
        gridArea: 'aside',
        boxSizing: 'border-box',
        position: 'relative',
        '@media (min-width: 64rem)': {
            width: "var(".concat(panelSplitterResizingVar, ", var(").concat(asideVar, "))"),
            justifySelf: 'end'
        }
    },
    inner: {
        insetBlockStart: contentInsetBlockStart,
        overflow: 'auto',
        height: '100%',
        '@media (min-width: 64rem)': {
            height: contentHeightWhenFixed,
            position: 'sticky'
        }
    }
});
function AsideComponent(param) {
    var children = param.children;
    return /*#__PURE__*/ React.createElement("aside", {
        css: styles.root,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/cssmap-class-names-in-css/in.jsx",
            lineNumber: 31,
            columnNumber: 3
        },
        __self: this
    }, /*#__PURE__*/ React.createElement("div", {
        css: styles.inner,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/cssmap-class-names-in-css/in.jsx",
            lineNumber: 32,
            columnNumber: 4
        },
        __self: this
    }, children));
}
