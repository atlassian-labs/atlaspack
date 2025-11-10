var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _css = require("@atlaskit/css");
var _this = undefined;
var styles = (0, _css.cssMap)({
    container: {
        paddingTop: 'var(--ds-space-100,8px)',
        paddingRight: 'var(--ds-space-100,8px)',
        paddingBottom: 'var(--ds-space-100,8px)',
        '&:hover': {
            backgroundColor: 'var(--ds-background-neutral-hovered,#091e4224)',
            cursor: 'pointer'
        }
    },
    fieldName: {
        flexGrow: 1
    }
});
var labelStyles = (0, _css.css)({
    display: 'block',
    width: '100%',
    cursor: 'pointer'
});
var Component = function() {
    return /*#__PURE__*/ React.createElement("label", {
        css: labelStyles,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/xcss-css-map-atlaskit/in.jsx",
            lineNumber: 25,
            columnNumber: 3
        },
        __self: _this
    }, /*#__PURE__*/ React.createElement("div", {
        xcss: styles.container,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/xcss-css-map-atlaskit/in.jsx",
            lineNumber: 26,
            columnNumber: 5
        },
        __self: _this
    }, "content"));
};
