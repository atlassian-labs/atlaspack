/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import React from 'react';
import { cssMap, cx, jsx } from '@compiled/react';
import { token } from './tokens';

const anchorStyles = cssMap({
  root: {
    display: 'flex',
    alignItems: 'center',
    height: '32px',
    borderRadius: token('radius.small', '3px'),
  },
  newInteractionStates: {
    '&:hover': {
      backgroundColor: token('color.background.neutral.subtle.hovered'),
    },
    '&:active': {
      backgroundColor: `${token('color.background.neutral.subtle.pressed')}!important`,
    },
  },
});

const logoContainerStyles = cssMap({
  root: {
    display: 'none',
    maxWidth: 320,
    boxSizing: 'content-box',
    paddingInline: token('space.100'),
    '@media (min-width: 64rem)': {
      '&&': {
        display: 'flex',
      },
    },
  },
});

const LogoRenderer = ({ logoOrIcon }) => {
  return <div>{logoOrIcon}</div>;
};

const Anchor = ({ children, xcss, ...props }) => {
  return <a {...props}>{children}</a>;
};

export const CustomLogo = ({ href, logo, icon, onClick, label }) => {
  return (
    <Anchor
      aria-label={label}
      href={href}
      xcss={cx(
        anchorStyles.root,
        anchorStyles.newInteractionStates,
      )}
      onClick={onClick}
    >
      <div
        css={[
          logoContainerStyles.root,
        ]}
      >
        <LogoRenderer logoOrIcon={logo} />
      </div>
    </Anchor>
  );
};