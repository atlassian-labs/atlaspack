import { css, jsx } from '@compiled/react';

const bodyStyles = css({
  display: 'flex',
  flexDirection: 'column',
  padding: '16px',
});

const imageStyles = css({
  display: 'block',
});

const defaultHeaderStyles = css({
  display: 'flex',
  alignItems: 'baseline',
  justifyContent: 'space-between',
});

const DefaultHeader = ({ children }) => (
  <div css={defaultHeaderStyles}>{children}</div>
);

function Component() {
  return (
    <div>
      <div css={bodyStyles}>Body content</div>
      <img css={imageStyles} src="test.jpg" alt="" />
      <DefaultHeader>Header</DefaultHeader>
    </div>
  );
}

export default Component;