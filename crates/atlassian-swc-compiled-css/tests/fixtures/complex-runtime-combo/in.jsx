import {
  css as compiledCss,
  styled as compiledStyled,
  keyframes,
  ClassNames,
  cssMap,
} from '@compiled/react';

const fade = keyframes`
  from { opacity: 0; }
  to { opacity: 1; }
`;

const toneMap = cssMap({
  primary: {
    color: 'dodgerblue',
  },
  danger: {
    color: 'crimson',
  },
});

const baseStyles = compiledCss({
  fontSize: '14px',
  '@media (min-width: 600px)': {
    '&:hover': {
      content: '"hover"',
      backgroundColor: 'black',
    },
  },
});

const Wrapper = compiledStyled.div({
  padding: 8,
  ':focus &': {
    outline: 'none',
  },
});

export const Component = () => (
  <ClassNames>
    {({ css }) => (
      <Wrapper css={[baseStyles, toneMap.primary]}>
        <span className={css({
          animation: `${fade} 2s linear`,
          ':after': {
            content: '"!"',
          },
        })}>
          combo
        </span>
      </Wrapper>
    )}
  </ClassNames>
);
