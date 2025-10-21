const _ = '._1wyb19bv{font-size:10px}';
const _2 =
  '@media (min-width:600px){._syaz13q2{color:blue}._syaz5scu{color:red}}';
import {ax} from '@compiled/react/runtime';
import {forwardRef} from 'react';
const C = forwardRef((props, __cmplr) => {
  const {as: __cmplC = 'div', style: __cmpls, ...__cmplp} = props;
  if (__cmplp.innerRef)
    throw new Error("Please use 'ref' instead of 'innerRef'.");
  return (
    <__cmplC
      {...__cmplp}
      style={__cmpls}
      ref={__cmplr}
      className={ax([
        '_1wyb19bv',
        __cmplp.isPrimary ? '_syaz13q2' : '_syaz5scu',
        __cmplp.className,
      ])}
    />
  );
});
if (process.env.NODE_ENV !== 'production') {
  C.displayName = 'C';
}
export const View = () => <C />;
