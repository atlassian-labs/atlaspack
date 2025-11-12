/**
 * @jsxRuntime classic
 * @jsx jsx
 */ /** @jsxFrag */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
var _objectSpread = require("@swc/helpers/_/_object_spread");
var _objectSpreadProps = require("@swc/helpers/_/_object_spread_props");
var _objectWithoutProperties = require("@swc/helpers/_/_object_without_properties");
var _css = require("@atlaskit/css");
var _dropdownMenu = require("@atlaskit/dropdown-menu");
var _dropdownMenuDefault = parcelHelpers.interopDefault(_dropdownMenu);
var _tokens = require("@atlaskit/tokens");
function Item(param) {
    var item = param.item, onItemClick = param.onItemClick, currentOpenedSubMenu = param.currentOpenedSubMenu, onSubMenuOpenChanged = param.onSubMenuOpenChanged;
    if ('children' in item && item.children) {
        var handleOpenChange = function(param) {
            var isOpen = param.isOpen;
            onSubMenuOpenChanged === null || onSubMenuOpenChanged === void 0 ? void 0 : onSubMenuOpenChanged(item, isOpen);
        };
        return /*#__PURE__*/ (0, _css.jsx)((0, _dropdownMenuDefault.default), {
            placement: "right-start",
            shouldRenderToParent: true,
            isOpen: currentOpenedSubMenu === item,
            onOpenChange: handleOpenChange,
            trigger: function(_param) {
                var triggerRef = _param.triggerRef, triggerProps = (0, _objectWithoutProperties._)(_param, [
                    "triggerRef"
                ]);
                return /*#__PURE__*/ (0, _css.jsx)((0, _dropdownMenu.DropdownItem), (0, _objectSpreadProps._)((0, _objectSpread._)({}, triggerProps), {
                    ref: triggerRef,
                    elemBefore: item.icon,
                    elemAfter: /*#__PURE__*/ (0, _css.jsx)("span", {
                        color: (0, _tokens.token)('color.icon.subtle'),
                        __source: {
                            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/refine-dropdown-jsx-error/in.jsx",
                            lineNumber: 28,
                            columnNumber: 8
                        }
                    }, "\u2192"),
                    __source: {
                        fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/refine-dropdown-jsx-error/in.jsx",
                        lineNumber: 23,
                        columnNumber: 6
                    }
                }), /*#__PURE__*/ (0, _css.jsx)("span", {
                    __source: {
                        fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/refine-dropdown-jsx-error/in.jsx",
                        lineNumber: 31,
                        columnNumber: 7
                    }
                }, item.title));
            },
            __source: {
                fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/refine-dropdown-jsx-error/in.jsx",
                lineNumber: 17,
                columnNumber: 4
            },
            __self: this
        }, /*#__PURE__*/ (0, _css.jsx)((0, _dropdownMenu.DropdownItemGroup), {
            __source: {
                fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/refine-dropdown-jsx-error/in.jsx",
                lineNumber: 35,
                columnNumber: 5
            },
            __self: this
        }, /*#__PURE__*/ (0, _css.jsx)(RefineDropdownItems, {
            items: item.children,
            onItemClick: onItemClick,
            __source: {
                fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/refine-dropdown-jsx-error/in.jsx",
                lineNumber: 36,
                columnNumber: 6
            },
            __self: this
        })));
    }
    return /*#__PURE__*/ (0, _css.jsx)((0, _dropdownMenu.DropdownItem), {
        elemBefore: item.icon,
        onClick: function() {
            return onItemClick();
        },
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/refine-dropdown-jsx-error/in.jsx",
            lineNumber: 43,
            columnNumber: 3
        },
        __self: this
    }, item.title);
}
