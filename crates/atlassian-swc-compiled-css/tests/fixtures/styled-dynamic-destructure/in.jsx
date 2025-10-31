import { styled } from '@compiled/react';

const Wrapper = styled.div({
  width: ({ width }) => width,
  transition: ({ duration }) => `width ${duration}ms ease`,
  flexShrink: 0,
});

export const Component = () => <Wrapper width="120px" duration={200} />;
