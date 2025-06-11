import Spinner from '@atlaskit/spinner';

export function DefaultLoadingIndicator() {
  return (
    <div
      style={{
        display: 'flex',
        paddingTop: 100,
        flexDirection: 'column',
        gap: 8,
        alignItems: 'center',
        height: '100%',
        width: '100%',
      }}
    >
      <Spinner size="xlarge" />
      <h2>Loading cache stats...</h2>
    </div>
  );
}
