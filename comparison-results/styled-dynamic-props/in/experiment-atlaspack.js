var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _this = undefined;
var Dot = (0, _react.styled).div({
    top: "".concat(function(props) {
        return props.y;
    }, "px"),
    left: "".concat(function(props) {
        return props.x;
    }, "px"),
    position: 'absolute',
    borderRadius: '9999px',
    width: '10px',
    height: '10px',
    transform: 'translate(-5px, -5px)',
    backgroundColor: 'blue'
});
var Component = function() {
    return /*#__PURE__*/ React.createElement(Dot, {
        x: 0,
        y: 0,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-dynamic-props/in.jsx",
            lineNumber: 14,
            columnNumber: 32
        },
        __self: _this
    });
};
