import React from 'react';
import ReactDOM from 'react-dom';
import AppRoutes from './ui/AppRoutes';
import {BrowserRouter} from 'react-router';
import {QueryClient, QueryClientProvider} from '@tanstack/react-query';

const queryClient = new QueryClient();

ReactDOM.render(
  <QueryClientProvider client={queryClient}>
    <BrowserRouter>
      <AppRoutes />
    </BrowserRouter>
  </QueryClientProvider>,
  document.getElementById('root'),
);
