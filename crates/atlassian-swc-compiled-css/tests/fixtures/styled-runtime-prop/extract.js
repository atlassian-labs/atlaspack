const _ = '._1bsb1gt8{width:var(--1vcp0mh)}';
const _2 = '._1ul91gt8{min-width:var(--1vcp0mh)}';
const _3 = '._p12f1gt8{max-width:var(--1vcp0mh)}';
import {ax, ix} from '@compiled/react/runtime';
import {forwardRef} from 'react';
export const BadgeSkeleton = forwardRef((props, __cmplr) => {
  const {as: C = 'span', style: __cmpls, ...__cmplp} = props;
  if (__cmplp.innerRef)
    throw new Error("Please use 'ref' instead of 'innerRef'.");
  return (
    <C
      {...__cmplp}
      style={{
        ...__cmpls,
        '--1vcp0mh': ix(__cmplp.width),
      }}
      ref={__cmplr}
      className={ax(['_1bsb1gt8 _1ul91gt8 _p12f1gt8', __cmplp.className])}
    />
  );
});
if (process.env.NODE_ENV !== 'production') {
  BadgeSkeleton.displayName = 'BadgeSkeleton';
}
export const View = () => <BadgeSkeleton />;
