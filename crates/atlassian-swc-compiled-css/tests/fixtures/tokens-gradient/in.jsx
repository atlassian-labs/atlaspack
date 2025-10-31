import { styled } from '@compiled/react';
import { token } from '@atlaskit/tokens';

const Box = styled.div({
	backgroundImage: `linear-gradient(
    to right,
    ${token('color.background.neutral')} 10%,
    ${token('color.background.neutral.subtle')} 30%,
    ${token('color.background.neutral')} 50%
  )`,
});

export const Component = () => <Box />;
