var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Example", function() {
    return Example;
});
var _react = require("@compiled/react");
var _tokens = require("@atlaskit/tokens");
var _this = undefined;
var DotStart = (0, _react.styled).div({
    position: 'absolute',
    top: function(props) {
        return "".concat(props.y, "px");
    },
    left: function(props) {
        return "".concat(props.x, "px");
    },
    borderRadius: (0, _tokens.token)('radius.full'),
    width: '10px',
    height: '10px',
    transform: 'translate(-5px, -5px)',
    backgroundColor: (0, _tokens.token)('color.background.accent.blue.subtler')
});
var DotEnd = (0, _react.styled)(DotStart)({
    backgroundColor: (0, _tokens.token)('color.background.accent.red.subtler')
});
var Example = function() {
    return /*#__PURE__*/ React.createElement(DotEnd, {
        x: 10,
        y: 20,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-dynamic-absolute-position/in.jsx",
            lineNumber: 19,
            columnNumber: 30
        },
        __self: _this
    });
};
