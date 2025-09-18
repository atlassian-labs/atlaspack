import {useSearchParams} from 'react-router';

import * as styles from './AdvancedSettings.module.css';

export function AdvancedSettings() {
  const [searchParams, setSearchParams] = useSearchParams();
  const bundle = searchParams.get('bundle');
  const isDetailView = bundle != null;
  const maxLevels = searchParams.get('maxLevels') ?? 0;
  const stacking = searchParams.get('stacking') ?? 'hierarchical';

  return (
    <div className={styles.advancedSettings}>
      <div className={styles.advancedSettingsInner}>
        <label className={styles.advancedSettingsLabel}>
          Max levels: {maxLevels}
        </label>

        <input
          disabled={!isDetailView}
          type="range"
          min={0}
          max={10}
          value={maxLevels}
          onChange={(e) =>
            setSearchParams((prev) => {
              prev.set('maxLevels', e.target.value);
              return prev;
            })
          }
        />
      </div>

      <div className={styles.advancedSettingsInner}>
        <label className={styles.advancedSettingsLabel}>Stacking</label>

        <select
          disabled={!isDetailView}
          value={stacking}
          onChange={(e) =>
            setSearchParams((prev) => {
              prev.set('stacking', e.target.value);
              return prev;
            })
          }
        >
          <option value="hierarchical">Hierarchical</option>
          <option value="flattened">Flattened</option>
        </select>
      </div>
    </div>
  );
}
