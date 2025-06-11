import {useParams} from 'react-router';
import {useQuery} from '@tanstack/react-query';
import {formatBytes} from '../../../formatBytes';
import {Code, CodeBlock} from '@atlaskit/code';
import {Box, Inline, Stack} from '@atlaskit/primitives';

export function CacheValuePage() {
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
        <Inline space="space.100">
          <Box>Cache entry size</Box>
          <Code>{formatBytes(cacheValue.size)}</Code>
        </Inline>

        <CodeBlock text={cacheValue.value} />
      </>
    );
  };

  return (
    <Stack space="space.100">
      <Box>
        <strong>Cache entry</strong>
      </Box>
      <Box>
        <Code>{key}</Code>
      </Box>

      {content()}
    </Stack>
  );
}
