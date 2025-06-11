import {useSuspenseQuery} from '@tanstack/react-query';
import {formatBytes} from './formatBytes';
import {Box, Stack} from '@atlaskit/primitives';

export function Stats() {
  const {data} = useSuspenseQuery<{
    size: number;
    keySize: number;
    count: number;
    assetContentCount: number;
    assetContentSize: number;
    assetMapCount: number;
    assetMapSize: number;
  }>({
    queryKey: ['/api/stats'],
  });

  return (
    <Box padding="space.100">
      <Stack space="space.100">
        <Box>
          <h1>Atlaspack cache stats</h1>
        </Box>

        <table>
          <tbody>
            <tr>
              <td>
                <strong>Cache size</strong>
              </td>
              <td>{formatBytes(data.size)}</td>
            </tr>
            <tr>
              <td>
                <strong>Total size of all keys</strong>
              </td>
              <td>{formatBytes(data.keySize)}</td>
            </tr>
            <tr>
              <td>
                <strong>Number of cache entries</strong>
              </td>
              <td>{data.count}</td>
            </tr>

            <tr>
              <td>
                <strong>Number of asset content entries</strong>
              </td>
              <td>{data.assetContentCount}</td>
            </tr>
            <tr>
              <td>
                <strong>Total size of all asset content entries</strong>
              </td>
              <td>{formatBytes(data.assetContentSize)}</td>
            </tr>

            <tr>
              <td>
                <strong>Number of asset map entries</strong>
              </td>
              <td>{data.assetMapCount}</td>
            </tr>
            <tr>
              <td>
                <strong>Total size of all asset map entries</strong>
              </td>
              <td>{formatBytes(data.assetMapSize)}</td>
            </tr>
          </tbody>
        </table>
      </Stack>
    </Box>
  );
}
