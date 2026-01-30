import {cssMap} from '@atlaskit/css';
import {Box} from '@atlaskit/primitives/compiled';
import {token} from '@atlaskit/tokens';

const styles = cssMap({
  pickerContainerStyle: {position: 'relative'},
  dropdownIndicatorStyles: {
    minWidth: '1.5rem',
    minHeight: '1.5rem',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  iconContainerStyles: {
    display: 'flex',
    height: '100%',
    position: 'absolute',
    alignItems: 'center',
    flexBasis: 'inherit',
    color: token('color.text.subtlest'),
    insetBlockStart: token('space.0'),
    insetInlineEnd: token('space.0'),
    transition: `color 150ms`,
    '&:hover': {
      color: token('color.text.subtle'),
    },
  },
  iconSpacingWithClearButtonStyles: {
    marginInlineEnd: token('space.400'),
  },
  iconSpacingWithoutClearButtonStyles: {
    marginInlineEnd: token('space.050'),
  },
});

console.log(styles);

export const Component = () => {
  return <Box xcss={styles.iconContainerStyles}>Hello world!</Box>;
};
