import { styled } from '@compiled/react';
import { token } from '@atlaskit/tokens';

const SkeletonRow = styled.div<{ height: number; width: number }>({
	backgroundImage: `linear-gradient(
    to right,
    ${token('color.background.neutral')} 10%,
    ${token('color.background.neutral.subtle')} 30%,
    ${token('color.background.neutral')} 50%
  )`,
	backgroundRepeat: 'no-repeat',
	// eslint-disable-next-line @atlaskit/ui-styling-standard/no-dynamic-styles -- fixture coverage
	height: (props) => `${props.height}px`,
	// eslint-disable-next-line @atlaskit/ui-styling-standard/no-dynamic-styles -- fixture coverage
	width: (props) => `${props.width}px`,
	borderRadius: token('radius.small', '3px'),
});

export const Component = () => <SkeletonRow height={40} width={200} />;
