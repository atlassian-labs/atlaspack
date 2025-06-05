import {Routes, Route} from 'react-router';
import {CacheValue} from './CacheValue';
import {Stats} from './Stats';
import {Bundles} from './Bundles';
import {Treemap} from './Treemap';
import {FoamTreemap} from './FoamTreemap';
import {SigmaPage} from './Sigma';
import {CytoscapePage} from './Cytoscape';
import {AppLayout} from './AppLayout';
import {CacheKeysPage} from './CacheKeysPage';
import {NotFoundPage} from './NotFoundPage';

export default function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<AppLayout />}>
        <Route index element={<Stats />} />
        <Route path="/app/bundles" element={<Bundles />} />
        <Route path="/app/treemap" element={<Treemap />} />
        <Route path="/app/sigma" element={<SigmaPage />} />
        <Route path="/app/cytoscape" element={<CytoscapePage />} />
        <Route path="/app/cache/" element={<CacheKeysPage />} />
        <Route path="/app/cache/:key" element={<CacheValue />} />
        <Route path="/app/foamtreemap" element={<FoamTreemap />} />

        <Route path="*" element={<NotFoundPage />} />
      </Route>
    </Routes>
  );
}
