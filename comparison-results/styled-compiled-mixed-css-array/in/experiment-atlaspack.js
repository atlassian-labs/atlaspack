/** @jsx jsx */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
var _taggedTemplateLiteral = require("@swc/helpers/_/_tagged_template_literal");
var _react = require("react");
var _reactDefault = parcelHelpers.interopDefault(_react);
var _react1 = require("@compiled/react");
var _styledComponents = require("styled-components");
var _styledComponentsDefault = parcelHelpers.interopDefault(_styledComponents);
var _this = undefined;
function _templateObject() {
    var data = (0, _taggedTemplateLiteral._)([
        "\n	",
        ";\n\n	&:focus,\n	&:hover {\n		background-color: #f4f5f7;\n		color: #333;\n		text-decoration: none;\n	}\n\n	&:active {\n		background-color: #e4e5ea;\n	}\n"
    ]);
    _templateObject = function _templateObject() {
        return data;
    };
    return data;
}
function _templateObject1() {
    var data = (0, _taggedTemplateLiteral._)([
        "\n	display: flex;\n	align-items: center;\n	padding: 2px 6px;\n	border-radius: 3px;\n	background-color: #f7f8f9;\n"
    ]);
    _templateObject1 = function _templateObject() {
        return data;
    };
    return data;
}
function _templateObject2() {
    var data = (0, _taggedTemplateLiteral._)([
        "\n	",
        ";\n	",
        ";\n"
    ]);
    _templateObject2 = function _templateObject() {
        return data;
    };
    return data;
}
// Mixed patterns: css2 from @compiled/react and css from styled-components
var referencedObjectsContainerStyles = (0, _react1.css)({
    display: 'flex',
    flexWrap: 'wrap',
    gap: '4px',
    maxWidth: '100%'
});
var maxWidth2 = (0, _react1.css)({
    maxWidth: '200px',
    overflow: 'hidden'
});
var plainTextStyles = (0, _react1.css)({
    display: 'flex',
    alignItems: 'center',
    paddingTop: '2px',
    paddingRight: 0,
    paddingBottom: '2px',
    paddingLeft: 0,
    marginRight: '4px',
    backgroundColor: 'inherit',
    color: '#333'
});
// Styled component using css template literal
var LozengeLink = (0, _styledComponentsDefault.default).a(_templateObject(), lozengeStyles);
var lozengeStyles = (0, _styledComponents.css)(_templateObject1());
// Component using @compiled/react jsx and css
var Component = function(param) {
    var children = param.children, forceMaxWidth = param.forceMaxWidth;
    return /*#__PURE__*/ (0, _react1.jsx)("div", {
        css: [
            plainTextStyles,
            forceMaxWidth && maxWidth2
        ],
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-compiled-mixed-css-array/in.jsx",
            lineNumber: 57,
            columnNumber: 2
        },
        __self: _this
    }, children);
};
// Component using styled-components
var StyledComponent = (0, _styledComponentsDefault.default).div(_templateObject2(), plainTextStyles, function(props) {
    return props.forceMaxWidth && maxWidth2;
});
exports.default = Component;
