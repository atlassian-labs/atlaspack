import './hack-feature-flags';
import './ui/globals.css';
import {createRoot} from 'react-dom/client';
import {BrowserRouter} from 'react-router';
import {
  QueryClient,
  QueryClientProvider,
  QueryFunction,
} from '@tanstack/react-query';
import axios, {AxiosError} from 'axios';

import AppRoutes from './AppRoutes';
import {APIError} from './APIError';

const defaultQueryFn: QueryFunction = async ({queryKey}) => {
  const backendUrl = process.env.ATLASPACK_INSPECTOR_BACKEND_URL;
  try {
    const {data} = await axios.get(`${backendUrl}${queryKey[0]}`);
    return data;
  } catch (err) {
    if (err instanceof AxiosError) {
      throw new APIError(err);
    }

    throw err;
  }
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
