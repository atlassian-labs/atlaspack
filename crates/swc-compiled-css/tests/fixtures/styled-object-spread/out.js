const _7 = '._k48p1693{font-weight:900}';
const _6 = '._11q71x77{background:white}';
const _5 = '._syaz5scu{color:red}';
const _4 = '._k48pni7l{font-weight:600}';
const _3 = '._syazbf54{color:green}';
const _2 = '._k48p1nn1{font-weight:400}';
const _ = '._syaz11x8{color:black}';
import { ax, CC, CS } from '@compiled/react/runtime';
import { forwardRef } from 'react';
const styles = {
	default: css({
		color: 'black',
		fontWeight: 400,
	}),
	success: css({
		color: 'green',
		fontWeight: 600,
	}),
	fail: css({
		color: 'red',
		fontWeight: 600,
	}),
	bg: css({
		background: 'white',
		fontWeight: 900,
	}),
};
const Component = forwardRef((props, __cmplr) => {
	const { as: C = 'div', style: __cmpls, ...__cmplp } = props;
	if (__cmplp.innerRef) throw new Error("Please use 'ref' instead of 'innerRef'.");
	return (
		<CC>
			<CS>{[_, _2, _6, _7]}</CS>
			{
				<C
					{...__cmplp}
					style={__cmpls}
					ref={__cmplr}
					className={ax(['_syaz11x8 _k48p1nn1 _11q71x77 _k48p1693', __cmplp.className])}
				/>
			}
		</CC>
	);
});
if (process.env.NODE_ENV !== 'production') {
	Component.displayName = 'Component';
}
export const View = () => <Component />;
