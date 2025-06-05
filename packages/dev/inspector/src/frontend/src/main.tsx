import './hack-feature-flags';
import './ui/globals.css';
import React from 'react';
import {createRoot} from 'react-dom/client';
import AppRoutes from './ui/AppRoutes';
import {BrowserRouter} from 'react-router';
import {
  QueryClient,
  QueryClientProvider,
  QueryFunction,
} from '@tanstack/react-query';
import axios from 'axios';

const defaultQueryFn: QueryFunction = async ({queryKey}) => {
  const {data} = await axios.get(`http://localhost:3000${queryKey[0]}`);
  return data;
};

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      queryFn: defaultQueryFn,
    },
  },
});

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error('Failed to find the root element');
}

const root = createRoot(rootElement);

root.render(
  <QueryClientProvider client={queryClient}>
    <BrowserRouter>
      <AppRoutes />
    </BrowserRouter>
  </QueryClientProvider>,
);
