var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _css = require("@atlaskit/css");
var _this = undefined;
var styles = (0, _css.cssMap)({
    linkField: {
        display: 'flex',
        alignItems: 'baseline',
        justifyContent: 'start',
        gap: 'var(--ds-space-050,4px)',
        color: 'var(--ds-text-subtle,#44546f)',
        paddingTop: 'var(--ds-space-100,8px)',
        paddingBottom: 'var(--ds-space-100,8px)',
        '> button': {
            alignSelf: 'end'
        }
    }
});
var Component = function() {
    return /*#__PURE__*/ React.createElement("div", {
        xcss: styles.linkField,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/css-map-child-selector/in.jsx",
            lineNumber: 18,
            columnNumber: 32
        },
        __self: _this
    }, "content");
};
