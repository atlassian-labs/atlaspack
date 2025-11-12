var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _css = require("@atlaskit/css");
var _this = undefined;
var styles = (0, _css.cssMap)({
    base: {
        color: 'var(--ds-text-subtle,#44546f)',
        paddingTop: 'var(--ds-space-100,8px)'
    },
    extra: {
        marginBottom: 'var(--ds-space-200,1pc)'
    }
});
var Component = function(param) {
    var showExtra = param.showExtra;
    return /*#__PURE__*/ React.createElement("div", {
        xcss: (0, _css.cx)(styles.base, showExtra ? styles.extra : null),
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/xcss-cx-css-map/in.jsx",
            lineNumber: 14,
            columnNumber: 3
        },
        __self: _this
    }, "content");
};
