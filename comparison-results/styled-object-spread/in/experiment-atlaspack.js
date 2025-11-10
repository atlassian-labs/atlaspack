var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _objectSpread = require("@swc/helpers/_/_object_spread");
var _objectSpreadProps = require("@swc/helpers/_/_object_spread_props");
var _react = require("@compiled/react");
var _tokens = require("@atlaskit/tokens");
var _this = undefined;
var layoutStyles = {
    alignItems: 'center',
    display: 'flex',
    gap: "".concat((0, _tokens.token)('space.100'))
};
var checkboxStyles = (0, _objectSpreadProps._)((0, _objectSpread._)({}, layoutStyles), {
    borderBottom: "".concat((0, _tokens.token)('space.025'), " solid ").concat((0, _tokens.token)('color.border')),
    height: "".concat((0, _tokens.token)('space.500')),
    width: '100%'
});
var getBackgroundColor = function(checked, disabled) {
    if (disabled) return (0, _tokens.token)('color.background.accent.gray.subtlest');
    return checked ? (0, _tokens.token)('color.background.accent.blue.subtlest') : 'transparent';
};
var ProjectCheckbox = (0, _react.styled).div((0, _objectSpreadProps._)((0, _objectSpread._)({}, checkboxStyles), {
    backgroundColor: function(param) {
        var checked = param.checked, disabled = param.disabled;
        return getBackgroundColor(checked, disabled);
    },
    input: {
        marginTop: (0, _tokens.token)('space.0'),
        marginRight: (0, _tokens.token)('space.075'),
        marginBottom: (0, _tokens.token)('space.0'),
        marginLeft: (0, _tokens.token)('space.075')
    },
    label: (0, _objectSpread._)({}, layoutStyles),
    p: {
        marginTop: (0, _tokens.token)('space.0'),
        marginRight: (0, _tokens.token)('space.0'),
        marginBottom: (0, _tokens.token)('space.0'),
        marginLeft: (0, _tokens.token)('space.0')
    }
}));
var Component = function() {
    return /*#__PURE__*/ React.createElement(ProjectCheckbox, {
        checked: true,
        disabled: false,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-object-spread/in.jsx",
            lineNumber: 47,
            columnNumber: 2
        },
        __self: _this
    }, /*#__PURE__*/ React.createElement("label", {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-object-spread/in.jsx",
            lineNumber: 48,
            columnNumber: 3
        },
        __self: _this
    }, /*#__PURE__*/ React.createElement("input", {
        type: "checkbox",
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-object-spread/in.jsx",
            lineNumber: 49,
            columnNumber: 4
        },
        __self: _this
    }), /*#__PURE__*/ React.createElement("p", {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-object-spread/in.jsx",
            lineNumber: 50,
            columnNumber: 4
        },
        __self: _this
    }, "Description")));
};
