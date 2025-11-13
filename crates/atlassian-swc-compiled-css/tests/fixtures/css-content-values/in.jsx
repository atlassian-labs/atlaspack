import { css } from '@compiled/react';

const styles = css({
	'._01 &': { content: 'none' },
	'._02 &': { content: 'url("http://www.example.com/test.png")' },
	'._03 &': { content: 'linear-gradient(#e66465, #9198e5)' },
	'._04 &': { content: 'image-set("image1x.png" 1x, "image2x.png" 2x)' },
	'._05 &': { content: 'url("http://www.example.com/test.png") / "This is the alt text"' },
	'._06 &': { content: '"prefix"' },
	'._07 &': { content: 'counter(chapter_counter)' },
	'._08 &': { content: 'counter(chapter_counter, upper-roman)' },
	'._09 &': { content: 'counters(section_counter, ".")' },
	'._10 &': { content: 'counters(section_counter, ".", decimal-leading-zero)' },
	'._11 &': { content: 'attr(value string)' },
	'._12 &': { content: 'open-quote' },
	'._13 &': { content: 'close-quote' },
	'._14 &': { content: 'no-open-quote' },
	'._15 &': { content: 'no-close-quote' },
	'._16 &': { content: 'open-quote counter(chapter_counter)' },
	'._17 &': { content: 'inherit' },
	'._18 &': { content: 'initial' },
	'._19 &': { content: 'revert' },
	'._20 &': { content: 'unset' },
	'._22 &': { content: '"✨"' },
	'._21 &': { content: '' },
});

<div css={styles} />;
