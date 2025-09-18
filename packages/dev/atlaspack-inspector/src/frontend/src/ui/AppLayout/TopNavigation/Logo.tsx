import atlaspackBadge from './badge-light.png';
import * as styles from './Logo.module.css';

export function Logo() {
  return (
    <div className={styles.logo}>
      <img src={atlaspackBadge} alt="Atlaspack" />
      <span className={styles.logoText}>Atlaspack</span>
    </div>
  );
}
