import styles from './GraphContainer.module.css';

export function GraphContainer({
  children,
  fullWidth = false,
}: {
  children: React.ReactNode;
  fullWidth?: boolean;
}) {
  return (
    <div
      className={styles.graphContainer}
      style={{
        width: fullWidth ? '100%' : 300,
      }}
    >
      {children}
    </div>
  );
}
