var _react = require("@compiled/react");
var stylesNew = (0, _react.cssMap)({
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
    'bg.bold.default': {
        backgroundColor: '#DDDEE1'
    },
    'bg.bold.inprogress': {
        backgroundColor: '#8FB8F6'
    },
    'bg.bold.moved': {
        backgroundColor: '#F9C84E'
    },
    'bg.bold.new': {
        backgroundColor: '#D8A0F7'
    },
    'bg.bold.removed': {
        backgroundColor: '#FD9891'
    },
    'bg.bold.success': {
        backgroundColor: '#B3DF72'
    },
    'bg.subtle.default': {
        backgroundColor: '#F4F5F7'
    },
    'bg.subtle.inprogress': {
        backgroundColor: '#F4F5F7'
    },
    'bg.subtle.moved': {
        backgroundColor: '#F4F5F7'
    },
    'bg.subtle.new': {
        backgroundColor: '#F4F5F7'
    },
    'bg.subtle.removed': {
        backgroundColor: '#F4F5F7'
    },
    'bg.subtle.success': {
        backgroundColor: '#F4F5F7'
    },
    'border.subtle.default': {
        border: '1px solid #B7B9BE'
    },
    'border.subtle.inprogress': {
        border: '1px solid #669DF1'
    },
    'border.subtle.moved': {
        border: '1px solid #FCA700'
    },
    'border.subtle.new': {
        border: '1px solid #C97CF4'
    },
    'border.subtle.removed': {
        border: '1px solid #F87168'
    },
    'border.subtle.success': {
        border: '1px solid #94C748'
    },
    'outline.subtle.default': {
        outline: '1px solid #B7B9BE'
    },
    'outline.subtle.inprogress': {
        outline: '1px solid #669DF1'
    },
    'outline.subtle.moved': {
        outline: '1px solid #FCA700'
    },
    'outline.subtle.new': {
        outline: '1px solid #C97CF4'
    },
    'outline.subtle.removed': {
        outline: '1px solid #F87168'
    },
    'outline.subtle.success': {
        outline: '1px solid #94C748'
    },
    'text.subtle': {
        color: '#42526E'
    },
    'text.bold': {
        color: '#292A2E'
    }
});
function LozengeComponent(param) {
    var appearance = param.appearance, isBold = param.isBold;
    var appearanceStyle = isBold ? 'bold' : 'subtle';
    return /*#__PURE__*/ React.createElement("span", {
        css: [
            stylesNew.container,
            stylesNew["bg.".concat(appearanceStyle, ".").concat(appearance)],
            stylesNew["border.subtle.".concat(appearance)],
            stylesNew.containerSubtle
        ],
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/lozenge-complex-cssmap/in.jsx",
            lineNumber: 59,
            columnNumber: 5
        },
        __self: this
    }, /*#__PURE__*/ React.createElement("span", {
        css: [
            stylesNew.text,
            stylesNew["text.".concat(appearanceStyle)]
        ],
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/lozenge-complex-cssmap/in.jsx",
            lineNumber: 67,
            columnNumber: 7
        },
        __self: this
    }, "Content"));
}
