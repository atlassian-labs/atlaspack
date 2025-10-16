const _ = '._syazbf54{color:green}';
const styles = null;
const cond = true;
<>
	<div xcss={styles} />
	<div
		xcss={
			cond
				? styles
				: css({
						color: 'blue',
				  })
		}
	/>
	<div xcss={Math.random() > 0.5 ? styles : styles} />
</>;
