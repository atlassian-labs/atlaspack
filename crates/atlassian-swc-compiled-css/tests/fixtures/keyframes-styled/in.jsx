import { styled, keyframes } from '@compiled/react';

const fade = keyframes({ from: { opacity: 0 }, to: { opacity: 1 } });

const Button = styled.button({
	animationName: fade,
});

console.log(Button);
