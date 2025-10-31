import { styled } from '@compiled/react';

export const Component = styled.div({
  '>:is(div, button)': {
    flexShrink: 0,
  },
});
