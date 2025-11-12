/** @jsx jsx */
import { jsx, css } from '@compiled/react';

const bodyStyles = css({
  display: 'flex',
  flexDirection: 'column',
});

const imageStyles = css({
  display: 'block',
});

const defaultHeaderStyles = css({
  display: 'flex',
  alignItems: 'baseline',
});

const Component = () => (
  <div css={bodyStyles}>
    <img css={imageStyles} src="test.jpg" alt="" />
    <div css={defaultHeaderStyles}>Header</div>
  </div>
);

export default Component;