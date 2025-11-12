import DropdownMenu, { DropdownItem, DropdownItemGroup } from '@atlaskit/dropdown-menu';
import { jsx } from '@atlaskit/css';
import { jsx } from "react/jsx-runtime";
function Item({ item, onItemClick, currentOpenedSubMenu, onSubMenuOpenChanged }) {
    if ('children' in item && item.children) {
        const handleOpenChange = ({ isOpen })=>{
            onSubMenuOpenChanged?.(item, isOpen);
        };
        return jsx(DropdownMenu, {
            placement: "right-start",
            shouldRenderToParent: true,
            isOpen: currentOpenedSubMenu === item,
            onOpenChange: handleOpenChange,
            trigger: ({ triggerRef, ...triggerProps })=>jsx(DropdownItem, {
                    ...triggerProps,
                    ref: triggerRef,
                    elemBefore: item.icon,
                    elemAfter: jsx("span", {
                        color: "var(--ds-icon-subtle, #626F86)",
                        children: "→"
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
        onClick: ()=>onItemClick(),
        children: item.title
    });
}
