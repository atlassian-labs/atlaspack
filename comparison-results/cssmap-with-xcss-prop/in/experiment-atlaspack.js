var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("react");
var _reactDefault = parcelHelpers.interopDefault(_react);
var _css = require("@atlaskit/css");
var _compiled = require("@atlaskit/primitives/compiled");
var _this = undefined;
var styles = (0, _css.cssMap)({
    avatarItemWrapper: {
        marginLeft: '-6px',
        paddingRight: '8px'
    },
    container: {
        display: 'flex',
        alignItems: 'center',
        backgroundColor: '#f4f5f7'
    },
    text: {
        fontSize: '14px',
        fontWeight: 'bold',
        color: '#172b4d'
    }
});
var Component = function(param) {
    var name = param.name, picture = param.picture;
    return /*#__PURE__*/ (0, _reactDefault.default).createElement((0, _compiled.Box), {
        xcss: styles.avatarItemWrapper,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/cssmap-with-xcss-prop/in.jsx",
            lineNumber: 24,
            columnNumber: 3
        },
        __self: _this
    }, /*#__PURE__*/ (0, _reactDefault.default).createElement("div", {
        className: styles.container(),
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/cssmap-with-xcss-prop/in.jsx",
            lineNumber: 25,
            columnNumber: 4
        },
        __self: _this
    }, /*#__PURE__*/ (0, _reactDefault.default).createElement("img", {
        src: picture,
        alt: name,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/cssmap-with-xcss-prop/in.jsx",
            lineNumber: 26,
            columnNumber: 5
        },
        __self: _this
    }), /*#__PURE__*/ (0, _reactDefault.default).createElement("span", {
        className: styles.text(),
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/cssmap-with-xcss-prop/in.jsx",
            lineNumber: 27,
            columnNumber: 5
        },
        __self: _this
    }, name)));
};
exports.default = Component;
