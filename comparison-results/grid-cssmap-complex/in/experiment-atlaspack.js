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
    },
    'space400': {
        rowGap: '32px'
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
    },
    'space400': {
        columnGap: '32px'
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
    },
    spaceBetween: {
        justifyContent: 'space-between'
    }
});
var alignContentMap = (0, _react1.cssMap)({
    start: {
        alignContent: 'flex-start'
    },
    center: {
        alignContent: 'center'
    },
    end: {
        alignContent: 'flex-end'
    },
    spaceBetween: {
        alignContent: 'space-between'
    }
});
var alignItemsMap = (0, _react1.cssMap)({
    start: {
        alignItems: 'flex-start'
    },
    center: {
        alignItems: 'center'
    },
    baseline: {
        alignItems: 'baseline'
    },
    end: {
        alignItems: 'flex-end'
    }
});
var baseStyles = (0, _react1.cssMap)({
    root: {
        display: 'grid',
        boxSizing: 'border-box'
    }
});
var gridAutoFlowMap = (0, _react1.cssMap)({
    row: {
        gridAutoFlow: 'row'
    },
    column: {
        gridAutoFlow: 'column'
    },
    dense: {
        gridAutoFlow: 'dense'
    }
});
/**
 * __Grid__
 *
 * `Grid` is a primitive component that implements the CSS Grid API.
 */ var Grid = function(props) {
    var tmp = props.as, Component = tmp === void 0 ? 'div' : tmp, alignItems = props.alignItems, alignContent = props.alignContent, justifyContent = props.justifyContent, gap = props.gap, columnGap = props.columnGap, rowGap = props.rowGap, children = props.children, id = props.id, autoFlow = props.autoFlow;
    return /*#__PURE__*/ (0, _reactDefault.default).createElement(Component, {
        id: id,
        className: "\n				".concat(baseStyles.root(), " \n				").concat(gap ? columnGapMap[gap]() : '', "\n				").concat(columnGap ? columnGapMap[columnGap]() : '', "\n				").concat(gap ? rowGapMap[gap]() : '', "\n				").concat(rowGap ? rowGapMap[rowGap]() : '', "\n				").concat(alignItems ? alignItemsMap[alignItems]() : '', "\n				").concat(alignContent ? alignContentMap[alignContent]() : '', "\n				").concat(justifyContent ? justifyContentMap[justifyContent]() : '', "\n				").concat(autoFlow ? gridAutoFlowMap[autoFlow]() : '', "\n			").trim(),
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/grid-cssmap-complex/in.jsx",
            lineNumber: 72,
            columnNumber: 3
        },
        __self: _this
    }, children);
};
Grid.displayName = 'Grid';
exports.default = Grid;
