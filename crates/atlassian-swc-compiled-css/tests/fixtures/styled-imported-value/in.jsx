import { styled } from '@compiled/react';
import { colors } from './palette';

const StyledDiv = styled.div({
  color: colors.primary,
});

export const Component = () => <StyledDiv>Hi</StyledDiv>;
