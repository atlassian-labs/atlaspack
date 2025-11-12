/**
 * @jsxRuntime classic
 * @jsx jsx
 */
/** @jsxFrag */
import { jsx } from '@atlaskit/css';
import DropdownMenu, { DropdownItem, DropdownItemGroup } from '@atlaskit/dropdown-menu';
import { token } from '@atlaskit/tokens';

function Item({ item, onItemClick, currentOpenedSubMenu, onSubMenuOpenChanged }) {
	if ('children' in item && item.children) {
		const handleOpenChange = ({ isOpen }) => {
			onSubMenuOpenChanged?.(item, isOpen);
		};

		return (
			<DropdownMenu
				placement="right-start"
				shouldRenderToParent
				isOpen={currentOpenedSubMenu === item}
				onOpenChange={handleOpenChange}
				trigger={({ triggerRef, ...triggerProps }) => (
					<DropdownItem
						{...triggerProps}
						ref={triggerRef}
						elemBefore={item.icon}
						elemAfter={
							<span color={token('color.icon.subtle')}>→</span>
						}
					>
						<span>{item.title}</span>
					</DropdownItem>
				)}
			>
				<DropdownItemGroup>
					<RefineDropdownItems items={item.children} onItemClick={onItemClick} />
				</DropdownItemGroup>
			</DropdownMenu>
		);
	}

	return (
		<DropdownItem elemBefore={item.icon} onClick={() => onItemClick()}>
			{item.title}
		</DropdownItem>
	);
}