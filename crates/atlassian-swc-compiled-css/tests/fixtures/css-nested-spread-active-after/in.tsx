import { css } from '@compiled/react';

const underline = {
  '&::after': {
    position: 'absolute',
    borderRadius: '1px',
  },
};

const pressed = {
  '&::after': {
    ...underline['&::after'],
    backgroundColor: 'red',
  },
};

const styles = css({
  '&:hover, &:focus-visible, &:focus': underline,
  '&:active': pressed,
});

export const Component = () => <div css={styles} />;
