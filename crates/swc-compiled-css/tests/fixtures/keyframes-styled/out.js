const _ = '@keyframes k3cx9vr0skpt4i{0%{opacity:0}to{opacity:1}}';
const _2 = '._j7hqjfxf{animation-name:k3cx9vr0skpt4i}';
import {ax, CC, CS} from '@compiled/react/runtime';
import {forwardRef} from 'react';
const fade = null;
const Button = forwardRef((props, __cmplr) => {
  const {as: C = 'button', style: __cmpls, ...__cmplp} = props;
  if (__cmplp.innerRef)
    throw new Error("Please use 'ref' instead of 'innerRef'.");
  return (
    <CC>
      <CS>{[_2]}</CS>
      {
        <C
          {...__cmplp}
          style={__cmpls}
          ref={__cmplr}
          className={ax(['_j7hqjfxf', __cmplp.className])}
        />
      }
    </CC>
  );
});
if (process.env.NODE_ENV !== 'production') {
  Button.displayName = 'Button';
}
console.log(Button);
