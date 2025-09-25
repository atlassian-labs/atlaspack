import {useSuspenseQuery} from '@tanstack/react-query';
import {useParams} from 'react-router';

export function CacheInvalidationFilePage() {
  const {fileId} = useParams();

  if (!fileId) {
    throw new Error('Invalid request, missing file ID');
  }

  const {data} = useSuspenseQuery({
    queryKey: [`/api/cache-invalidation-files/${encodeURIComponent(fileId)}`],
  });

  return (
    <div>
      <h1>Cache invalidation file {fileId}</h1>

      <pre>{JSON.stringify(data, null, 2)}</pre>
    </div>
  );
}
