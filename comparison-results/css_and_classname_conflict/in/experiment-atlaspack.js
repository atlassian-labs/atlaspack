var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Banner", function() {
    return Banner;
});
var _react = require("@compiled/react");
var styles = (0, _react.cssMap)({
    root: {
        gridArea: 'banner',
        height: 'var(--banner-height)',
        insetBlockStart: 0,
        position: 'sticky',
        zIndex: 100,
        overflow: 'hidden'
    }
});
function Banner(param) {
    var xcss = param.xcss, testId = param.testId, id = param.id;
    return /*#__PURE__*/ React.createElement("div", {
        id: id,
        css: styles.root,
        className: xcss,
        "data-testid": testId,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/css_and_classname_conflict/in.jsx",
            lineNumber: 16,
            columnNumber: 9
        },
        __self: this
    }, "Content");
}
