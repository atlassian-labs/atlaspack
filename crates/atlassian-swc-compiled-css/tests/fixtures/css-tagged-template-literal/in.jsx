import { css } from '@compiled/react';

const styles = css`
  color: orange;
  &:hover {
    color: black;
  }
`;

export const Component = () => <div css={styles}>Hello</div>;
