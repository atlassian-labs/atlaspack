var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _this = undefined;
var styles = (0, _react.cssMap)({
    container: {
        display: 'inline-flex',
        borderRadius: '3px',
        blockSize: 'min-content',
        position: 'static',
        overflow: 'hidden',
        paddingInline: '4px',
        boxSizing: 'border-box'
    },
    text: {
        fontFamily: 'ui-sans-serif',
        fontSize: '11px',
        fontStyle: 'normal',
        fontWeight: 'bold',
        lineHeight: '16px',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        textTransform: 'uppercase',
        whiteSpace: 'nowrap'
    }
});
var Component = function() {
    return /*#__PURE__*/ React.createElement("div", {
        css: styles.container,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/cssmap-unbounded-basic/in.jsx",
            lineNumber: 27,
            columnNumber: 3
        },
        __self: _this
    }, /*#__PURE__*/ React.createElement("span", {
        css: styles.text,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/cssmap-unbounded-basic/in.jsx",
            lineNumber: 28,
            columnNumber: 5
        },
        __self: _this
    }, "Hello"));
};
