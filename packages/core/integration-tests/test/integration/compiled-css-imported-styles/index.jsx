import { css } from '@compiled/react';
import { primary } from './mixins/colors';

const styles = css({ color: primary });

const App = () => (
  <div css={styles}>hello from atlaspack</div>
);

export default App;
