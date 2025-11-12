var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
var _react = require("@compiled/react");
var styles = (0, _react.cssMap)({
    container: {
        display: 'inline-flex',
        boxSizing: 'border-box',
        position: 'static',
        blockSize: 'min-content',
        borderRadius: '3px',
        overflow: 'hidden',
        paddingInlineStart: '4px',
        paddingInlineEnd: '4px'
    },
    containerSubtle: {
        outlineOffset: -1
    },
    text: {
        fontFamily: 'ui-sans-serif',
        fontSize: '11px',
        fontStyle: 'normal',
        fontWeight: 'bold',
        lineHeight: '16px',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        textTransform: 'uppercase',
        whiteSpace: 'nowrap'
    },
    customLetterspacing: {
        letterSpacing: 0.165
    },
    bgBoldDefault: {
        backgroundColor: '#DDDEE1'
    },
    bgBoldInprogress: {
        backgroundColor: '#8FB8F6'
    },
    bgBoldMoved: {
        backgroundColor: '#F9C84E'
    },
    bgBoldNew: {
        backgroundColor: '#D8A0F7'
    },
    bgBoldRemoved: {
        backgroundColor: '#FD9891'
    },
    bgBoldSuccess: {
        backgroundColor: '#B3DF72'
    },
    bgSubtleDefault: {
        backgroundColor: '#F4F5F7'
    },
    bgSubtleInprogress: {
        backgroundColor: '#F4F5F7'
    },
    bgSubtleMoved: {
        backgroundColor: '#F4F5F7'
    },
    bgSubtleNew: {
        backgroundColor: '#F4F5F7'
    },
    bgSubtleRemoved: {
        backgroundColor: '#F4F5F7'
    },
    bgSubtleSuccess: {
        backgroundColor: '#F4F5F7'
    },
    borderSubtleDefault: {
        border: '1px solid #B7B9BE'
    },
    borderSubtleInprogress: {
        border: '1px solid #669DF1'
    },
    borderSubtleMoved: {
        border: '1px solid #FCA700'
    },
    borderSubtleNew: {
        border: '1px solid #C97CF4'
    },
    borderSubtleRemoved: {
        border: '1px solid #F87168'
    },
    borderSubtleSuccess: {
        border: '1px solid #94C748'
    },
    textSubtle: {
        color: '#172B4D'
    },
    textBold: {
        color: '#292A2E'
    }
});
function Lozenge(param) {
    var children = param.children, _param_isBold = param.isBold, isBold = _param_isBold === void 0 ? false : _param_isBold, _param_appearance = param.appearance, appearance = _param_appearance === void 0 ? 'default' : _param_appearance;
    var appearanceStyle = isBold ? 'Bold' : 'Subtle';
    var bgClass = "bg".concat(appearanceStyle).concat(appearance.charAt(0).toUpperCase() + appearance.slice(1));
    var textClass = "text".concat(appearanceStyle);
    return /*#__PURE__*/ React.createElement("span", {
        className: styles.container(),
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/lozenge-cssmap-complex/in.jsx",
            lineNumber: 59,
            columnNumber: 5
        },
        __self: this
    }, /*#__PURE__*/ React.createElement("span", {
        className: styles[bgClass](),
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/lozenge-cssmap-complex/in.jsx",
            lineNumber: 60,
            columnNumber: 7
        },
        __self: this
    }, /*#__PURE__*/ React.createElement("span", {
        className: styles.text(),
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/lozenge-cssmap-complex/in.jsx",
            lineNumber: 61,
            columnNumber: 9
        },
        __self: this
    }, /*#__PURE__*/ React.createElement("span", {
        className: styles.customLetterspacing(),
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/lozenge-cssmap-complex/in.jsx",
            lineNumber: 62,
            columnNumber: 11
        },
        __self: this
    }, /*#__PURE__*/ React.createElement("span", {
        className: styles[textClass](),
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/lozenge-cssmap-complex/in.jsx",
            lineNumber: 63,
            columnNumber: 13
        },
        __self: this
    }, children)))));
}
exports.default = Lozenge;
