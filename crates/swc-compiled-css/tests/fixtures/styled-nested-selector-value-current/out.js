const _ = '._syaz13q2:hover{color:blue}';
const _2 = '._syaz5scu:hover{color:red}';
const _3 = '._1wyb1fwx{font-size:12px}';
import {ax, CC, CS} from '@compiled/react/runtime';
import {forwardRef} from 'react';
const C = forwardRef((props, __cmplr) => {
  const {as: __cmplC = 'div', style: __cmpls, ...__cmplp} = props;
  if (__cmplp.innerRef)
    throw new Error("Please use 'ref' instead of 'innerRef'.");
  return (
    <CC>
      <CS>{[_3]}</CS>
      {
        <__cmplC
          {...__cmplp}
          style={__cmpls}
          ref={__cmplr}
          className={ax([
            '_1wyb1fwx',
            __cmplp.isActive ? '_syaz13q2' : '_syaz5scu',
            __cmplp.className,
          ])}
        />
      }
    </CC>
  );
});
if (process.env.NODE_ENV !== 'production') {
  C.displayName = 'C';
}
export const View = () => <C />;
