import {useNavigate, useSearchParams} from 'react-router';
import styles from '../../../App.module.css';
import {useVirtualizer} from '@tanstack/react-virtual';
import {useEffect, useMemo, useRef} from 'react';
import {token} from '@atlaskit/tokens';
import {useInfiniteQuery} from '@tanstack/react-query';
import qs from 'qs';
import axios from 'axios';
import Button, {LinkButton} from '@atlaskit/button/new';
import { Box } from '@atlaskit/primitives';

export function CacheKeyList() {
  // sort by in querystring URL
  const [searchParams, setSearchParams] = useSearchParams();
  const sortBy = searchParams.get('sortBy') || 'order';
  const setSortBy = (value: string) => {
    searchParams.set('sortBy', value);
    setSearchParams(searchParams);
  };

  const {
    data: cacheKeys,
    isLoading,
    error,
    fetchNextPage,
    isFetchingNextPage,
    hasNextPage,
  } = useInfiniteQuery<{
    keys: string[];
    count: number;
    hasNextPage: boolean;
    nextPageCursor: string | null;
  }>({
    queryFn: async ({pageParam}) => {
      const {data} = await axios.get(
        'http://localhost:3000/api/cache-keys/?' +
          qs.stringify({sortBy, cursor: pageParam}),
      );
      return data;
    },
    queryKey: ['/api/cache-keys/?' + qs.stringify({sortBy})],
    initialPageParam: null,
    getNextPageParam: (lastPage) => lastPage.nextPageCursor,
  });

  const allKeys = useMemo(
    () => cacheKeys?.pages.flatMap((page) => page.keys) ?? [],
    [cacheKeys?.pages],
  );
  const lastPage = cacheKeys?.pages[cacheKeys.pages.length - 1];
  const parentRef = useRef<HTMLDivElement>(null);
  const rowVirtualizer = useVirtualizer({
    count: lastPage?.hasNextPage ? allKeys.length + 1 : allKeys.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 35,
  });

  const virtualItems = rowVirtualizer.getVirtualItems();
  useEffect(() => {
    const lastVirtualItem = virtualItems[virtualItems.length - 1];
    const loaderRowIsShown =
      allKeys[lastVirtualItem?.index] == null && lastPage?.hasNextPage;

    if (hasNextPage && loaderRowIsShown && !isFetchingNextPage) {
      fetchNextPage();
    }
  }, [
    allKeys,
    allKeys.length,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    lastPage,
    lastPage?.hasNextPage,
    virtualItems,
  ]);

  const navigate = useNavigate();

  const renderItems = () => {
    if (isLoading) {
      return (
        <div
          style={{
            flex: 1,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          Loading...
        </div>
      );
    }

    if (error) {
      return (
        <div
          style={{
            flex: 1,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          Error: {error.message}
        </div>
      );
    }

    if (!allKeys.length) {
      return (
        <div
          style={{
            flex: 1,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          No cache keys
        </div>
      );
    }

    return (
      <div
        style={{
          flex: 1,
          height: '100%',
          width: '100%',
          overflowY: 'auto',
          position: 'relative',
        }}
        ref={parentRef}
      >
        <div
          style={{
            width: '100%',
            height: `${rowVirtualizer.getTotalSize()}px`,
          }}
        >
          {rowVirtualizer.getVirtualItems().map((virtualItem) => {
            const key = allKeys[virtualItem.index];
            const isLoaderRow = key == null && lastPage?.hasNextPage;

            if (isLoaderRow) {
              return (
                <div
                  key="loader"
                  className={styles.sidebarItem}
                  style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    width: '100%',
                    height: `${virtualItem.size}px`,
                    transform: `translateY(${virtualItem.start}px)`,
                  }}
                >
                  Loading...
                </div>
              );
            }
            const href = `/app/cache/${encodeURIComponent(
              key,
            )}?${searchParams.toString()}`;

            return (
              <div
                key={virtualItem.index}
                className={styles.sidebarItem}
                style={{
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  width: '100%',
                  height: `${virtualItem.size}px`,
                  transform: `translateY(${virtualItem.start}px)`,
                }}
              >
                <LinkButton
                  appearance="subtle"
                  href={href}
                  onClick={(e) => {
                    e.preventDefault();
                    navigate(href);
                  }}
                >
                  {key}
                </LinkButton>
              </div>
            );
          })}
        </div>
      </div>
    );
  };

  return (
    <div
      style={{
        padding: 8,
        height: '100%',
        gap: 8,
        display: 'flex',
        backgroundColor: token('elevation.surface.sunken'),
        flexDirection: 'column',
        border: '1px solid var(--ds-border)',
        width: 300,
        flexShrink: 0,
      }}
    >
      <Box>
        <Button
          appearance="subtle"
          onClick={() => {
            if (sortBy === 'order') {
              setSortBy('size');
            } else {
              setSortBy('order');
            }
          }}
        >
          Sorting by {sortBy}
        </Button>
      </Box>

      <div
        style={{
          backgroundColor: 'var(--ds-background-input)',
          padding: 4,
          border: '1px solid var(--ds-border)',
          borderRadius: 4,
          flex: 1,
          overflow: 'hidden',
          gap: 4,
        }}
      >
        {renderItems()}
      </div>
    </div>
  );
}
