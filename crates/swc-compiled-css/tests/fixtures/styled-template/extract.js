const _2 = '._syaz13q2:hover{color:blue}';
const _ = '._syaz5scu{color:red}';
import { ax } from '@compiled/react/runtime';
import { forwardRef } from 'react';
const Button = forwardRef((props, __cmplr) => {
	const { as: C = 'div', style: __cmpls, ...__cmplp } = props;
	if (__cmplp.innerRef) throw new Error("Please use 'ref' instead of 'innerRef'.");
	return (
		<C
			{...__cmplp}
			style={__cmpls}
			ref={__cmplr}
			className={ax(['_syaz5scu _syaz13q2', __cmplp.className])}
		/>
	);
});
if (process.env.NODE_ENV !== 'production') {
	Button.displayName = 'Button';
}
export const Btn = () => <Button />;
