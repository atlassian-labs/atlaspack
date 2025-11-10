/** @jsx jsx */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
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
    var children = param.children, xcss = param.xcss, _param_height = param.height, height = _param_height === void 0 ? 48 : _param_height, testId = param.testId;
    return /*#__PURE__*/ (0, _react.jsx)("div", {
        "data-layout-slot": true,
        css: styles.root,
        "data-testid": testId,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/banner-xcss-cssmap/in.jsx",
            lineNumber: 17,
            columnNumber: 5
        },
        __self: this
    }, children);
}
exports.default = Banner;
