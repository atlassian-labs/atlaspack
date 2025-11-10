/**
 * @jsxRuntime classic
 * @jsx jsx
 */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Banner", function() {
    return Banner;
});
var _react = require("@compiled/react");
var bannerMountedVar = '--n_bnrM';
var localSlotLayers = {
    banner: 4
};
var styles = (0, _react.cssMap)({
    root: {
        gridArea: 'banner',
        height: "var(".concat(bannerMountedVar, ")"),
        insetBlockStart: 0,
        position: 'sticky',
        zIndex: localSlotLayers.banner,
        overflow: 'hidden'
    }
});
function Banner(param) {
    var children = param.children, testId = param.testId;
    return /*#__PURE__*/ (0, _react.jsx)("div", {
        "data-layout-slot": true,
        css: styles.root,
        "data-testid": testId,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/banner-navigation-system/in.jsx",
            lineNumber: 25,
            columnNumber: 3
        },
        __self: this
    }, children);
}
