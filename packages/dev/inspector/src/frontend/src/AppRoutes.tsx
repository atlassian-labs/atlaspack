import {Routes, Route} from 'react-router';
import {CacheValuePage} from './ui/app/cache/[key]/CacheValuePage';
import {CacheKeysIndexPage} from './ui/app/cache/CacheKeysIndexPage';
import {StatsPage} from './ui/app/StatsPage';
import {FoamTreemapPage} from './ui/app/treemap/FoamTreemapPage';
import {AppLayout} from './ui/AppLayout';
import {CacheKeysPage} from './ui/app/cache/CacheKeysPage';
import {NotFoundPage} from './ui/not-found/NotFoundPage';

export default function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<AppLayout />}>
        <Route index element={<StatsPage />} />

        <Route path="/app/cache" element={<CacheKeysPage />}>
          <Route index element={<CacheKeysIndexPage />} />
          <Route path="/app/cache/:key" element={<CacheValuePage />} />
        </Route>

        <Route path="/app/treemap" element={<FoamTreemapPage />} />

        <Route path="*" element={<NotFoundPage />} />
      </Route>
    </Routes>
  );
}
