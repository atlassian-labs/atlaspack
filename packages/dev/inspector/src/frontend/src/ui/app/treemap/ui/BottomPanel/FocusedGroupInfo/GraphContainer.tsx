import {token} from '@atlaskit/tokens';

export function GraphContainer({
  children,
  fullWidth = false,
}: {
  children: React.ReactNode;
  fullWidth?: boolean;
}) {
  return (
    <div
      style={{
        height: 'calc(100% - 16px)',
        width: fullWidth ? '100%' : 300,
        border: '1px solid var(--ds-border)',
        borderRadius: '8px',
        backgroundColor: token('elevation.surface.sunken'),
        margin: '8px',
      }}
    >
      {children}
    </div>
  );
}
