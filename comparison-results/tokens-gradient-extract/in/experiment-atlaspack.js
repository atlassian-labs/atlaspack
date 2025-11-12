var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _tokens = require("@atlaskit/tokens");
var _this = undefined;
var SkeletonRow = (0, _react.styled).div({
    backgroundImage: "linear-gradient(\n    to right,\n    ".concat((0, _tokens.token)('color.background.neutral'), " 10%,\n    ").concat((0, _tokens.token)('color.background.neutral.subtle'), " 30%,\n    ").concat((0, _tokens.token)('color.background.neutral'), " 50%\n  )"),
    backgroundRepeat: 'no-repeat',
    // eslint-disable-next-line @atlaskit/ui-styling-standard/no-dynamic-styles -- fixture coverage
    height: function(props) {
        return "".concat(props.height, "px");
    },
    // eslint-disable-next-line @atlaskit/ui-styling-standard/no-dynamic-styles -- fixture coverage
    width: function(props) {
        return "".concat(props.width, "px");
    },
    borderRadius: (0, _tokens.token)('radius.small', '3px')
});
var Component = function() {
    return /*#__PURE__*/ React.createElement(SkeletonRow, {
        height: 40,
        width: 200,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/tokens-gradient-extract/in.tsx",
            lineNumber: 19,
            columnNumber: 32
        },
        __self: _this
    });
};
