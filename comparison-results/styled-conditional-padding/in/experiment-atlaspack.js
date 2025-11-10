var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _this = undefined;
var PaddingWrapper = (0, _react.styled).div({
    padding: "4px 8px 8px ".concat(function(param) {
        var isSummaryView = param.isSummaryView;
        return isSummaryView ? '0px' : '12px';
    })
});
var Component = function() {
    return /*#__PURE__*/ React.createElement(PaddingWrapper, {
        isSummaryView: false,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-conditional-padding/in.jsx",
            lineNumber: 8,
            columnNumber: 32
        },
        __self: _this
    }, "Content");
};
