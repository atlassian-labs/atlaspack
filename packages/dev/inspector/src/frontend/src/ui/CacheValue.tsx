import {useParams} from 'react-router';
import {useQuery} from '@tanstack/react-query';
import {formatBytes} from './formatBytes';

export function CacheValue() {
  const key = useParams().key;
  if (!key) {
    throw new Error('No key');
  }

  const {
    data: cacheValue,
    isLoading,
    error,
  } = useQuery<{
    size: number;
    value: string;
  }>({
    queryKey: [`/api/cache-value/${encodeURIComponent(key)}`],
  });

  const content = () => {
    if (isLoading) {
      return <div>Loading...</div>;
    }

    if (error) {
      return <div>Error: {error.message}</div>;
    }

    if (!cacheValue) {
      throw new Error('No cache value');
    }

    return (
      <>
        <p>size: {formatBytes(cacheValue.size)}</p>

        <pre>
          <code>{cacheValue.value}</code>
        </pre>
      </>
    );
  };

  return (
    <div>
      <h2>cache value</h2>
      <h3>key: {key}</h3>

      {content()}
    </div>
  );
}
