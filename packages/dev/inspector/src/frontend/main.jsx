import React from 'react';
import {createRoot} from 'react-dom/client';
import AppRoutes from './ui/AppRoutes';
import {BrowserRouter} from 'react-router';
import {QueryClient, QueryClientProvider} from '@tanstack/react-query';
import axios from 'axios';

const defaultQueryFn = async ({queryKey}) => {
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

const root = createRoot(document.getElementById('root'));

root.render(
  <QueryClientProvider client={queryClient}>
    <BrowserRouter>
      <AppRoutes />
    </BrowserRouter>
  </QueryClientProvider>,
);
