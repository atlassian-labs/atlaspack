const _ = '._syaz13q2{color:blue}';
const _2 = '._syaz5scu{color:red}';
const _3 = '._1hms1911{text-decoration-line:line-through}';
const _4 = '._1hmsglyw{text-decoration-line:none}';
const _5 = '._1yyj11wp{-webkit-line-clamp:3}';
const _6 = '._1yyjkb7n{-webkit-line-clamp:1}';
const _7 = '._1wyb1ul9{font-size:30px}';
import {ax, CC, CS} from '@compiled/react/runtime';
import {forwardRef} from 'react';
const Component = forwardRef((props, __cmplr) => {
  const {as: C = 'button', style: __cmpls, ...__cmplp} = props;
  if (__cmplp.innerRef)
    throw new Error("Please use 'ref' instead of 'innerRef'.");
  return (
    <CC>
      <CS>{[_7]}</CS>
      {
        <C
          {...__cmplp}
          style={__cmpls}
          ref={__cmplr}
          className={ax([
            '_1wyb1ul9',
            __cmplp.isPrimary ? '_syaz13q2' : '_syaz5scu',
            __cmplp.isDone ? '_1hms1911' : '_1hmsglyw',
            __cmplp.isClamped ? '_1yyj11wp' : '_1yyjkb7n',
            __cmplp.className,
          ])}
        />
      }
    </CC>
  );
});
if (process.env.NODE_ENV !== 'production') {
  Component.displayName = 'Component';
}
export const View = () => <Component />;
