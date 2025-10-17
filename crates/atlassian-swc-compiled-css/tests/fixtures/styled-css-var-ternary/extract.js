const _ = '._bfhk11x8{background-color:black}';
const _2 = '._syaz1x77{color:white}';
const _3 = '._bfhk1x77{background-color:white}';
const _4 = '._syaz11x8{color:black}';
const _5 = '._1wyb1ul9{font-size:30px}';
import {ax} from '@compiled/react/runtime';
import {forwardRef} from 'react';
const dark = null;
const light = null;
const Component = forwardRef((props, __cmplr) => {
  const {as: C = 'div', style: __cmpls, ...__cmplp} = props;
  if (__cmplp.innerRef)
    throw new Error("Please use 'ref' instead of 'innerRef'.");
  return (
    <C
      {...__cmplp}
      style={__cmpls}
      ref={__cmplr}
      className={ax([
        '_1wyb1ul9',
        __cmplp.isDark ? '_bfhk11x8 _syaz1x77' : '_bfhk1x77 _syaz11x8',
        __cmplp.className,
      ])}
    />
  );
});
if (process.env.NODE_ENV !== 'production') {
  Component.displayName = 'Component';
}
export const View = () => <Component />;
