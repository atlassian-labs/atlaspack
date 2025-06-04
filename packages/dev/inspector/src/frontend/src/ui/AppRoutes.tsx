import {Routes, Route} from 'react-router';
import App from './App';
import {CacheValue} from './CacheValue';
import {Stats} from './Stats';
import {Bundles} from './Bundles';
import {Treemap} from './Treemap';
import {FoamTreemap} from './FoamTreemap';
import {SigmaPage} from './Sigma';

export default function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<App />}>
        <Route index element={<Stats />} />
        <Route path="/app/bundles" element={<Bundles />} />
        <Route path="/app/treemap" element={<Treemap />} />
        <Route path="/app/foamtreemap" element={<FoamTreemap />} />
        <Route path="/app/sigma" element={<SigmaPage />} />
        <Route path="/app/cache/:key" element={<CacheValue />} />
      </Route>
    </Routes>
  );
}
