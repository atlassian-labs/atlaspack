const _ = '._syaz5scu{color:red}';
import {ax} from '@compiled/react/runtime';
import {forwardRef} from 'react';
const mixin = () => ({
  color: 'red',
});
const ListItem = forwardRef((props, __cmplr) => {
  const {as: C = 'div', style: __cmpls, ...__cmplp} = props;
  if (__cmplp.innerRef)
    throw new Error("Please use 'ref' instead of 'innerRef'.");
  return (
    <C
      {...__cmplp}
      style={__cmpls}
      ref={__cmplr}
      className={ax(['_syaz5scu', __cmplp.className])}
    />
  );
});
if (process.env.NODE_ENV !== 'production') {
  ListItem.displayName = 'ListItem';
}
export const View = () => <ListItem />;
