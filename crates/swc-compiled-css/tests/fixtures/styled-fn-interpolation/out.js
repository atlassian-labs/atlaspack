const _7 = '._11q71x77{background:white}';
const _6 = '._syaz5scu{color:red}';
const _5 = '._1h6d1gy6{border-color:yellow}';
const _4 = '._1dqonqa1{border-style:solid}';
const _3 = '._189e1l7b{border-width:3px}';
const _2 = '._11q713q2{background:blue}';
const _ = '._syaz13q2{color:blue}';
import { ax, CC, CS } from '@compiled/react/runtime';
import { forwardRef } from 'react';
const Component = forwardRef((props, __cmplr) => {
	const { as: C = 'div', style: __cmpls, ...__cmplp } = props;
	if (__cmplp.innerRef) throw new Error("Please use 'ref' instead of 'innerRef'.");
	return (
		<CC>
			<CS>{[_3, _4, _5, _6, _7]}</CS>
			{
				<C
					{...__cmplp}
					style={__cmpls}
					ref={__cmplr}
					className={ax([
						'_189e1l7b _1dqonqa1 _1h6d1gy6 _syaz5scu _11q71x77',
						__cmplp.isPrimary && '_syaz13q2 _11q713q2',
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
