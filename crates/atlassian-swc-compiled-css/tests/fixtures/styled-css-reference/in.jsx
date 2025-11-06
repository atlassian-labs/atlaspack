import { css, styled } from '@compiled/react';

const themedUnderline = {
  '&::after': {
    left: 0,
    content: "''",
  },
};

const tabStyles = css({
  display: 'flex',
  '&:active': {
    outline: 'none',
    ...themedUnderline,
  },
});

export const Tab = styled.div(tabStyles);
