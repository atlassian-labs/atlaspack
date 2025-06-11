import {useSearchParams} from 'react-router';

export function AdvancedSettings() {
  const [searchParams, setSearchParams] = useSearchParams();
  const bundle = searchParams.get('bundle');
  const isDetailView = bundle != null;
  const maxLevels = searchParams.get('maxLevels') ?? 0;
  const stacking = searchParams.get('stacking') ?? 'hierarchical';

  return (
    <div style={{padding: '8px'}}>
      <div style={{display: 'flex', flexDirection: 'column', gap: '10px'}}>
        <label style={{fontWeight: 'bold'}}>Max levels: {maxLevels}</label>
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

      <div style={{display: 'flex', flexDirection: 'column', gap: '10px'}}>
        <label style={{fontWeight: 'bold'}}>Stacking</label>
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
