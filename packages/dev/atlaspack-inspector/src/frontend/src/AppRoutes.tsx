import {Routes, Route} from 'react-router';
import {Suspense} from 'react';

import {CacheValuePage} from './ui/app/cache/[key]/CacheValuePage';
import {CacheKeysIndexPage} from './ui/app/cache/CacheKeysIndexPage';
import {StatsPage} from './ui/app/StatsPage';
import {FoamTreemapPage} from './ui/app/treemap/FoamTreemapPage';
import {AppLayout} from './ui/AppLayout/AppLayout';
import {CacheKeysPage} from './ui/app/cache/CacheKeysPage';
import {CacheInvalidationPage} from './ui/app/cache-invalidation/CacheInvalidationPage';
import {NotFoundPage} from './ui/not-found/NotFoundPage';
import {ErrorBoundary} from './ui/util/ErrorBoundary';
import {DefaultLoadingIndicator} from './ui/DefaultLoadingIndicator/DefaultLoadingIndicator';
import {CacheInvalidationFilePage} from './ui/app/cache-invalidation/[fileId]/CacheInvalidationFilePage';

/**
 * All the routes in the atlaspack-inspector app.
 */
export default function AppRoutes() {
  return (
    <Suspense
      fallback={
        <DefaultLoadingIndicator message="Loading atlaspack-inspector..." />
      }
    >
      <ErrorBoundary>
        <Routes>
          <Route path="/" element={<AppLayout />}>
            <Route index element={<FoamTreemapPage />} />

            <Route path="/app/cache-stats" element={<StatsPage />} />
            <Route path="/app/cache" element={<CacheKeysPage />}>
              <Route index element={<CacheKeysIndexPage />} />
              <Route path="/app/cache/:key" element={<CacheValuePage />} />
            </Route>

            <Route
              path="/app/cache-invalidation"
              element={<CacheInvalidationPage />}
            >
              <Route index element={null} />
              <Route
                path="/app/cache-invalidation/:fileId"
                element={<CacheInvalidationFilePage />}
              />
            </Route>

            <Route path="/app/treemap" element={<FoamTreemapPage />} />

            <Route path="*" element={<NotFoundPage />} />
          </Route>
        </Routes>
      </ErrorBoundary>
    </Suspense>
  );
}
