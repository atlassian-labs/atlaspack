var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _taggedTemplateLiteral = require("@swc/helpers/_/_tagged_template_literal");
var _react = require("@compiled/react");
var _this = undefined;
function _templateObject() {
    var data = (0, _taggedTemplateLiteral._)([
        "\n  from { opacity: 0; }\n  to { opacity: 1; }\n"
    ]);
    _templateObject = function _templateObject() {
        return data;
    };
    return data;
}
var fade = (0, _react.keyframes)(_templateObject());
var toneMap = (0, _react.cssMap)({
    primary: {
        color: 'dodgerblue'
    },
    danger: {
        color: 'crimson'
    }
});
var baseStyles = (0, _react.css)({
    fontSize: '14px',
    '@media (min-width: 600px)': {
        '&:hover': {
            content: '"hover"',
            backgroundColor: 'black'
        }
    }
});
var Wrapper = (0, _react.styled).div({
    padding: 8,
    ':focus &': {
        outline: 'none'
    }
});
var Component = function() {
    return /*#__PURE__*/ React.createElement((0, _react.ClassNames), {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/complex-runtime-combo/in.jsx",
            lineNumber: 41,
            columnNumber: 3
        },
        __self: _this
    }, function(param) {
        var css = param.css;
        return /*#__PURE__*/ React.createElement(Wrapper, {
            css: [
                baseStyles,
                toneMap.primary
            ],
            __source: {
                fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/complex-runtime-combo/in.jsx",
                lineNumber: 43,
                columnNumber: 7
            },
            __self: _this
        }, /*#__PURE__*/ React.createElement("span", {
            className: css({
                animation: "".concat(fade, " 2s linear"),
                ':after': {
                    content: '"!"'
                }
            }),
            __source: {
                fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/complex-runtime-combo/in.jsx",
                lineNumber: 44,
                columnNumber: 9
            },
            __self: _this
        }, "combo"));
    });
};
