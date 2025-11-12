import { cssMap } from '@compiled/react';

export const styles = cssMap({
  success: {
    color: '#0b0',
    '&:hover': {
      color: '#060',
    },
    '@media': {
      'screen and (min-width: 500px)': {
        fontSize: '10vw',
      },
    },
    selectors: {
      span: {
        color: 'lightgreen',
        '&:hover': {
          color: '#090',
        },
      },
    },
  },
  danger: {
    color: 'red',
    '&:hover': {
      color: 'darkred',
    },
    '@media': {
      'screen and (min-width: 500px)': {
        fontSize: '20vw',
      },
    },
    selectors: {
      span: {
        color: 'orange',
        '&:hover': {
          color: 'pink',
        },
      },
    },
  },
});
