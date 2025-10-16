const _5 = '._4t3iwi6j{height:var(--3yhx9g)}';
const _4 = '._p12f1gt8{max-width:var(--1vcp0mh)}';
const _3 = '._1ul91gt8{min-width:var(--1vcp0mh)}';
const _2 = '._1bsb1ynz{width:var(--1ea5ebz)}';
const _ = '._bfhk32ev{background-color:pink}';
import { ax, ix, CC, CS } from '@compiled/react/runtime';
import { forwardRef } from 'react';
export const Complex = forwardRef((props, __cmplr) => {
	const { as: C = 'div', style: __cmpls, ...__cmplp } = props;
	if (__cmplp.innerRef) throw new Error("Please use 'ref' instead of 'innerRef'.");
	return (
		<CC>
			<CS>{[_, _2, _3, _4, _5]}</CS>
			{
				<C
					{...__cmplp}
					style={{
						...__cmpls,
						'--1ea5ebz': ix(
							(() => {
								return __cmplp.dim.width + 10 + 'px';
							})(),
						),
						'--1vcp0mh': ix(__cmplp.width),
						'--3yhx9g': ix(__cmplp.dim.height),
					}}
					ref={__cmplr}
					className={ax(['_bfhk32ev _1bsb1ynz _1ul91gt8 _p12f1gt8 _4t3iwi6j', __cmplp.className])}
				/>
			}
		</CC>
	);
});
if (process.env.NODE_ENV !== 'production') {
	Complex.displayName = 'Complex';
}
export const View = () => <Complex />;
