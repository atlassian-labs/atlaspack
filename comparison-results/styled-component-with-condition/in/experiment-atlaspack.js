var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _jiraFeatureFlaggingUtils = require("@atlassian/jira-feature-flagging-utils");
var _motion = require("@atlaskit/motion");
var _this = undefined;
var OuterWrapperOld = (0, _react.styled).div({
    display: 'flex',
    contain: 'layout',
    width: function(param) {
        var width = param.width;
        return width;
    },
    transition: function(param) {
        var duration = param.duration;
        return "width ".concat(duration, "ms ").concat((0, _motion.easeInOut));
    }
});
var OuterWrapperNew = (0, _react.styled).div({
    display: 'flex',
    contain: 'layout',
    width: function(param) {
        var width = param.width;
        return width;
    },
    transition: function(param) {
        var duration = param.duration;
        return "width ".concat(duration, "ms ").concat((0, _motion.easeInOut));
    },
    overflow: 'hidden'
});
var OuterWrapper = (0, _jiraFeatureFlaggingUtils.componentWithCondition)(function() {
    return true;
}, OuterWrapperNew, OuterWrapperOld);
var InnerWrapper = (0, _react.styled).div({
    flexShrink: 0
});
var Component = function(param) {
    var width = param.width, duration = param.duration;
    return /*#__PURE__*/ React.createElement(OuterWrapper, {
        width: width,
        duration: duration,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-component-with-condition/in.jsx",
            lineNumber: 31,
            columnNumber: 3
        },
        __self: _this
    }, /*#__PURE__*/ React.createElement(InnerWrapper, {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-component-with-condition/in.jsx",
            lineNumber: 32,
            columnNumber: 5
        },
        __self: _this
    }));
};
