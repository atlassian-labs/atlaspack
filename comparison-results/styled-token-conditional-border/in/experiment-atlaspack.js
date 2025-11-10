var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("react");
var _reactDefault = parcelHelpers.interopDefault(_react);
var _react1 = require("@compiled/react");
var _tokens = require("@atlaskit/tokens");
var _this = undefined;
var Wrapper = (0, _react1.styled).div({
    border: function(param) {
        var isSummaryView = param.isSummaryView;
        return isSummaryView ? 'none' : "1px solid ".concat((0, _tokens.token)('color.border'));
    }
});
var Component = function() {
    return /*#__PURE__*/ (0, _reactDefault.default).createElement(Wrapper, {
        isSummaryView: false,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-token-conditional-border/in.jsx",
            lineNumber: 10,
            columnNumber: 32
        },
        __self: _this
    });
};
