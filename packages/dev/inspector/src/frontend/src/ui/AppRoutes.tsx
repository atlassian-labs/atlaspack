import {Routes, Route} from 'react-router';
import {CacheValuePage} from './app/cache/[key]/CacheValuePage';
import {CacheKeysIndexPage} from './app/cache/CacheKeysIndexPage';
import {Stats} from './Stats';
import {FoamTreemapPage} from './app/treemap/FoamTreemapPage';
import {AppLayout} from './AppLayout';
import {CacheKeysPage} from './app/cache/CacheKeysPage';
import {NotFoundPage} from './not-found/NotFoundPage';

export default function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<AppLayout />}>
        <Route index element={<Stats />} />

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
