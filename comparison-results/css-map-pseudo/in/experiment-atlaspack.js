var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _this = undefined;
var styles = (0, _react.cssMap)({
    base: {
        position: 'relative',
        margin: '8px',
        '&::before': {
            content: '',
            position: 'absolute',
            width: '100%',
            height: '100%'
        },
        '&:focus-within::before': {
            boxShadow: 'inset 0 0 0 2px blue'
        }
    }
});
var Component = function() {
    return /*#__PURE__*/ React.createElement("div", {
        css: styles.base,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/css-map-pseudo/in.jsx",
            lineNumber: 19,
            columnNumber: 32
        },
        __self: _this
    }, "pseudo");
};
