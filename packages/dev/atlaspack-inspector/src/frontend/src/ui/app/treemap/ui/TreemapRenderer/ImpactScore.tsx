import * as styles from './ImpactScore.module.css';

export function ImpactScore({
  parentSize,
  groupSize,
  message,
}: {
  parentSize: number;
  groupSize: number;
  message: string;
}) {
  return (
    <div className={styles.impactScore}>
      <div
        className={styles.impactScoreBar}
        style={{width: `${Math.min(1, groupSize / parentSize) * 100}%`}}
      />

      <div className={styles.impactScoreMessage}>
        <div>{message}</div>
      </div>
    </div>
  );
}
