import './hack-feature-flags';
import './ui/globals.css';
import {createRoot} from 'react-dom/client';
import {BrowserRouter} from 'react-router';
import {
  QueryClient,
  QueryClientProvider,
  QueryFunction,
} from '@tanstack/react-query';
import axios from 'axios';

import AppRoutes from './AppRoutes';

const defaultQueryFn: QueryFunction = async ({queryKey}) => {
  const backendUrl = process.env.ATLASPACK_INSPECTOR_BACKEND_URL;
  const {data} = await axios.get(`${backendUrl}${queryKey[0]}`);
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
