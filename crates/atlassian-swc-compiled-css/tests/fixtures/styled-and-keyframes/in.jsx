import { keyframes, styled } from '@compiled/react';

const pulse = keyframes({
  '0%': {
    transform: 'scale(1)',
  },
  '50%': {
    transform: 'scale(1.1)',
  },
  '100%': {
    transform: 'scale(1)',
  },
});

const StyledDiv = styled.div({
  animation: `${pulse} 2s infinite`,
});

export const Component = () => <StyledDiv>Pulse</StyledDiv>;
