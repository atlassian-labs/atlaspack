var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _this = undefined;
var Wrapper = (0, _react.styled).div({
    width: function(param) {
        var width = param.width;
        return width;
    },
    transition: function(param) {
        var duration = param.duration;
        return "width ".concat(duration, "ms ease");
    },
    flexShrink: 0
});
var Component = function() {
    return /*#__PURE__*/ React.createElement(Wrapper, {
        width: "120px",
        duration: 200,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-dynamic-destructure/in.jsx",
            lineNumber: 9,
            columnNumber: 32
        },
        __self: _this
    });
};
