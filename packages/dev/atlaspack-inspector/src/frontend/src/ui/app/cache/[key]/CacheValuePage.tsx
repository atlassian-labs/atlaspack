import {useParams} from 'react-router';
import {useSuspenseQuery} from '@tanstack/react-query';
import {Code, CodeBlock} from '@atlaskit/code';
import {Box, Inline, Stack} from '@atlaskit/primitives';

import {formatBytes} from '../../../util/formatBytes';

interface CacheValueResponse {
  size: number;
  value: string;
}

export function CacheValuePage() {
  const key = useParams().key;
  if (!key) {
    throw new Error('No key');
  }

  const {data: cacheValue} = useSuspenseQuery<CacheValueResponse>({
    queryKey: [`/api/cache-value/${encodeURIComponent(key)}`],
  });

  return (
    <Stack space="space.100">
      <Box>
        <strong>Cache entry</strong>
      </Box>
      <Box>
        <Code>{key}</Code>
      </Box>

      <Inline space="space.100">
        <Box>Cache entry size</Box>
        <Code>{formatBytes(cacheValue.size)}</Code>
      </Inline>

      <CodeBlock text={cacheValue.value} />
    </Stack>
  );
}
