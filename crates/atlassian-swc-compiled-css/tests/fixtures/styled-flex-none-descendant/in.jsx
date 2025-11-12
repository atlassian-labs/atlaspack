import { styled } from '@compiled/react';

const Item = styled.div({
  flex: 'none',
  '&:hover': {
    '[data-target="child"]': {
      opacity: 1,
    },
  },
});

export const Component = () => (
  <Item>
    <span data-target="child">Child</span>
  </Item>
);
