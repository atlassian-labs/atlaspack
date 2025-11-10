var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
var _react = require("react");
var _reactDefault = parcelHelpers.interopDefault(_react);
var _react1 = require("@compiled/react");
var _this = undefined;
var rowGapMap = (0, _react1.cssMap)({
    'space100': {
        rowGap: '8px'
    },
    'space200': {
        rowGap: '16px'
    },
    'space300': {
        rowGap: '24px'
    }
});
var columnGapMap = (0, _react1.cssMap)({
    'space100': {
        columnGap: '8px'
    },
    'space200': {
        columnGap: '16px'
    },
    'space300': {
        columnGap: '24px'
    }
});
var justifyContentMap = (0, _react1.cssMap)({
    start: {
        justifyContent: 'flex-start'
    },
    center: {
        justifyContent: 'center'
    },
    end: {
        justifyContent: 'flex-end'
    }
});
var alignItemsMap = (0, _react1.cssMap)({
    start: {
        alignItems: 'flex-start'
    },
    center: {
        alignItems: 'center'
    },
    end: {
        alignItems: 'flex-end'
    }
});
var styles = (0, _react1.cssMap)({
    root: {
        display: 'flex',
        boxSizing: 'border-box'
    }
});
/**
 * __Flex__
 *
 * `Flex` is a primitive component that implements the CSS Flexbox API.
 */ var Flex = function(props) {
    var tmp = props.as, Component = tmp === void 0 ? 'div' : tmp, alignItems = props.alignItems, justifyContent = props.justifyContent, gap = props.gap, columnGap = props.columnGap, rowGap = props.rowGap, children = props.children;
    return /*#__PURE__*/ (0, _reactDefault.default).createElement(Component, {
        className: "\n				".concat(styles.root(), " \n				").concat(gap ? columnGapMap[gap]() : '', "\n				").concat(columnGap ? columnGapMap[columnGap]() : '', "\n				").concat(gap ? rowGapMap[gap]() : '', "\n				").concat(rowGap ? rowGapMap[rowGap]() : '', "\n				").concat(alignItems ? alignItemsMap[alignItems]() : '', "\n				").concat(justifyContent ? justifyContentMap[justifyContent]() : '', "\n			").trim(),
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/flex-cssmap-complex/in.jsx",
            lineNumber: 52,
            columnNumber: 3
        },
        __self: _this
    }, children);
};
Flex.displayName = 'Flex';
exports.default = Flex;
