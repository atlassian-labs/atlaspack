import { styled } from '@compiled/react';
import { colors } from './theme';

const Box = styled.div({
  backgroundColor: colors.N0,
  color: colors.N800,
});

export const Component = () => <Box>Hi</Box>;
