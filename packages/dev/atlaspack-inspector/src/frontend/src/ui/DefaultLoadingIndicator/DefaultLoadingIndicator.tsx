import Spinner from '@atlaskit/spinner';
import * as styles from './DefaultLoadingIndicator.module.css';

export function DefaultLoadingIndicator({message}: {message?: string}) {
  return (
    <div
      className={styles.defaultLoadingIndicator}
      data-testid="atlaspack-inspector-loading-indicator"
    >
      <Spinner size="xlarge" />
      <h2>{message ?? 'Loading cache stats...'}</h2>
    </div>
  );
}
