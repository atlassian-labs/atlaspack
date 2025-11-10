var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Styled", function() {
    return Styled;
});
var _defineProperty = require("@swc/helpers/_/_define_property");
var _react = require("react");
var _reactDefault = parcelHelpers.interopDefault(_react);
var _react1 = require("@compiled/react");
var _this = undefined;
var selectors = {
    helper: 'helper-class',
    id: 'target-id'
};
var _obj;
var Component = (0, _react1.styled).div((_obj = {
    color: 'rebeccapurple',
    backgroundColor: function(param) {
        var isActive = param.isActive;
        return isActive ? 'lime' : 'cyan';
    }
}, (0, _defineProperty._)(_obj, ".".concat(selectors.helper), {
    padding: '4px',
    color: 'tomato'
}), (0, _defineProperty._)(_obj, "#".concat(selectors.id), {
    marginTop: '8px',
    '&:hover': {
        opacity: 0.5
    }
}), _obj));
var Styled = function() {
    return /*#__PURE__*/ (0, _reactDefault.default).createElement(Component, {
        isActive: true,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-dynamic-computed-selector/in.jsx",
            lineNumber: 25,
            columnNumber: 3
        },
        __self: _this
    }, /*#__PURE__*/ (0, _reactDefault.default).createElement("span", {
        className: "helper-class",
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-dynamic-computed-selector/in.jsx",
            lineNumber: 26,
            columnNumber: 5
        },
        __self: _this
    }, "Helper"), /*#__PURE__*/ (0, _reactDefault.default).createElement("span", {
        id: "target-id",
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-dynamic-computed-selector/in.jsx",
            lineNumber: 27,
            columnNumber: 5
        },
        __self: _this
    }, "Target"));
};
