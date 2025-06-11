import Spinner from '@atlaskit/spinner';
import styles from './DefaultLoadingIndicator.module.css';

export function DefaultLoadingIndicator() {
  return (
    <div className={styles.defaultLoadingIndicator}>
      <Spinner size="xlarge" />
      <h2>Loading cache stats...</h2>
    </div>
  );
}
