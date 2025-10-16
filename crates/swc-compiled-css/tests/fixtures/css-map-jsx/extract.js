const _ = '._syaz5scu{color:red}';
const _2 = '._syaz13q2:hover{color:blue}';
const _3 = '._bfhkr75e{background-color:#eee}';
const _4 = '._syaz14zx{color:crimson}';
import {ax} from '@compiled/react/runtime';
const isActive = true;
const count = 1;
const themes = cssMap({
  primary: {
    color: 'red',
    '&:hover': {
      color: 'blue',
    },
  },
  secondary: {
    backgroundColor: '#eee',
  },
  danger: {
    color: 'crimson',
  },
});
<>
  <div className={ax(['_syaz5scu _syaz13q2'])} />
  <div className={ax(['_syaz5scu _syaz13q2', isActive && themes.secondary])} />
  <div
    className={ax([false && themes.secondary, '_syaz5scu _syaz13q2', cx('A')])}
  />
  <div
    className={ax(['_bfhkr75e', '_syaz14zx', count > 0 && themes.primary])}
  />
</>;
