import {AxiosError} from 'axios';

/**
 * A custom error class for HTTP errors.
 */
export class APIError extends Error {
  constructor(err: AxiosError) {
    const url = err.config?.url;
    const method = err.config?.method;
    const status = err.response?.status;
    const statusText = err.response?.statusText;
    const data = err.response?.data;
    const cause = err.cause;

    super(
      `Failed to fetch: ${[
        method?.toUpperCase(),
        url,
        status,
        statusText,
        cause?.message,
        '\n',
        JSON.stringify(data, null, 2),
      ]
        .filter(Boolean)
        .join(' ')}`,
    );
  }
}
