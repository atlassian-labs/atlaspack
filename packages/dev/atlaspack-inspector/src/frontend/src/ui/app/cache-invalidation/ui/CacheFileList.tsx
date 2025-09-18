import {useNavigate, useSearchParams} from 'react-router';
import {useVirtualizer} from '@tanstack/react-virtual';
import {useEffect, useMemo, useRef} from 'react';
import {useInfiniteQuery} from '@tanstack/react-query';
import qs from 'qs';
import axios from 'axios';
import Button, {LinkButton} from '@atlaskit/button/new';
import {Box} from '@atlaskit/primitives';

import * as styles from './CacheFileList.module.css';
import * as appStyles from '../../../App.module.css';

export function CacheFileList() {
  const [searchParams, setSearchParams] = useSearchParams();
  const sortBy = searchParams.get('sortBy') || 'invalidationCount';
  const setSortBy = (value: string) => {
    searchParams.set('sortBy', value);
    setSearchParams(searchParams);
  };

  const {
    data: cacheFiles,
    isLoading,
    error,
    fetchNextPage,
    isFetchingNextPage,
    hasNextPage,
  } = useInfiniteQuery<{
    files: {
      id: string;
    }[];
    count: number;
    hasNextPage: boolean;
    nextPageCursor: string | null;
  }>({
    queryFn: async ({pageParam}) => {
      const backendUrl = process.env.ATLASPACK_INSPECTOR_BACKEND_URL;
      const {data} = await axios.get(
        `${backendUrl}/api/cache-invalidation-files/?` +
          qs.stringify({cursor: pageParam, sortBy}),
      );
      return data;
    },
    queryKey: ['/api/cache-invalidation-files/?' + qs.stringify({sortBy})],
    initialPageParam: null,
    getNextPageParam: (lastPage) => lastPage.nextPageCursor,
  });

  const allFiles = useMemo(
    () => cacheFiles?.pages.flatMap((page) => page.files) ?? [],
    [cacheFiles?.pages],
  );
  const lastPage = cacheFiles?.pages[cacheFiles.pages.length - 1];
  const parentRef = useRef<HTMLDivElement>(null);
  const rowVirtualizer = useVirtualizer({
    count: lastPage?.hasNextPage ? allFiles.length + 1 : allFiles.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 35,
  });

  const virtualItems = rowVirtualizer.getVirtualItems();
  useEffect(() => {
    const lastVirtualItem = virtualItems[virtualItems.length - 1];
    const loaderRowIsShown =
      allFiles[lastVirtualItem?.index] == null && lastPage?.hasNextPage;

    if (hasNextPage && loaderRowIsShown && !isFetchingNextPage) {
      fetchNextPage();
    }
  }, [
    allFiles,
    allFiles.length,
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
        <div className={styles.cacheFilePlaceholderContainer}>Loading...</div>
      );
    }

    if (error) {
      return (
        <div className={styles.cacheFilePlaceholderContainer}>
          Error: {error.message}
        </div>
      );
    }

    if (!allFiles.length) {
      return (
        <div className={styles.cacheFilePlaceholderContainer}>
          No cache files
        </div>
      );
    }

    return (
      <div className={styles.cacheFileListItemsContainer} ref={parentRef}>
        <div
          className={styles.cacheFileListItemsContainerInner}
          style={{
            height: `${rowVirtualizer.getTotalSize()}px`,
          }}
        >
          {rowVirtualizer.getVirtualItems().map((virtualItem) => {
            const file = allFiles[virtualItem.index];
            const isLoaderRow = file == null && lastPage?.hasNextPage;
            const rowStyle = {
              position: 'absolute',
              top: 0,
              left: 0,
              width: '100%',
              height: `${virtualItem.size}px`,
              transform: `translateY(${virtualItem.start}px)`,
            } as const;

            if (isLoaderRow) {
              return (
                <div
                  key="loader"
                  className={appStyles.sidebarItem}
                  style={rowStyle}
                >
                  Loading...
                </div>
              );
            }
            const href = `/app/cache-invalidation/${encodeURIComponent(
              file.id,
            )}?${searchParams.toString()}`;

            return (
              <div
                key={virtualItem.index}
                className={appStyles.sidebarItem}
                style={rowStyle}
              >
                <LinkButton
                  appearance="subtle"
                  href={href}
                  onClick={(e) => {
                    e.preventDefault();
                    navigate(href);
                  }}
                >
                  {file.id}
                </LinkButton>
              </div>
            );
          })}
        </div>
      </div>
    );
  };

  return (
    <div className={styles.cacheFileList}>
      <Box>
        <Button
          appearance="subtle"
          onClick={() => {
            if (sortBy === 'order') {
              setSortBy('invalidationCount');
            } else {
              setSortBy('order');
            }
          }}
        >
          Sorting by {sortBy}
        </Button>
      </Box>

      <div className={styles.cacheFileListInner}>{renderItems()}</div>
    </div>
  );
}
