import './globals.css';
import React from 'react';
import App, {CacheValue, Stats} from './App';
import {Routes, Route} from 'react-router';

export default function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<App />}>
        <Route index element={<CacheValue />} />
        <Route path="/app/stats" element={<Stats />} />
        <Route path="/app/cache/:key" element={<CacheValue />} />
      </Route>
    </Routes>
  );
}
