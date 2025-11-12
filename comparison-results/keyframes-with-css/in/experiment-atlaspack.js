var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _this = undefined;
var fadeIn = (0, _react.keyframes)({
    from: {
        opacity: 0,
        transform: 'translateY(20px)'
    },
    to: {
        opacity: 1,
        transform: 'translateY(0)'
    }
});
var animatedStyles = (0, _react.css)({
    animation: "".concat(fadeIn, " 0.3s ease-in-out"),
    padding: '16px',
    backgroundColor: 'white',
    borderRadius: '4px'
});
var Component = function(param) {
    var children = param.children;
    return /*#__PURE__*/ React.createElement("div", {
        css: animatedStyles,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/keyframes-with-css/in.jsx",
            lineNumber: 22,
            columnNumber: 10
        },
        __self: _this
    }, children);
};
