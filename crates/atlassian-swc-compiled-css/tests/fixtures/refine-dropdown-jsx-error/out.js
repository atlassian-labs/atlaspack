/** @jsxFrag */
import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import DropdownMenu, { DropdownItem, DropdownItemGroup } from '@atlaskit/dropdown-menu';
import { token } from '@atlaskit/tokens';
import { jsx } from "react/jsx-runtime";
function Item({
  item,
  onItemClick,
  currentOpenedSubMenu,
  onSubMenuOpenChanged
}) {
  if ('children' in item && item.children) {
    const handleOpenChange = ({
      isOpen
    }) => {
      onSubMenuOpenChanged?.(item, isOpen);
    };
    return jsx(DropdownMenu, {
      placement: "right-start",
      shouldRenderToParent: true,
      isOpen: currentOpenedSubMenu === item,
      onOpenChange: handleOpenChange,
      trigger: ({
        triggerRef,
        ...triggerProps
      }) => jsx(DropdownItem, {
        ...triggerProps,
        ref: triggerRef,
        elemBefore: item.icon,
        elemAfter: jsx("span", {
          color: token('color.icon.subtle'),
          children: "\u2192"
        }),
        children: jsx("span", {
          children: item.title
        })
      }),
      children: jsx(DropdownItemGroup, {
        children: jsx(RefineDropdownItems, {
          items: item.children,
          onItemClick: onItemClick
        })
      })
    });
  }
  return jsx(DropdownItem, {
    elemBefore: item.icon,
    onClick: () => onItemClick(),
    children: item.title
  });
}
