/**
 * A custom error class for HTTP errors.
 *
 * The {@link errorHandlingMiddleware} will automatically set HTTP response
 * status codes for instances of this class.
 */
export class HTTPError extends Error {
  constructor(
    message: string,
    public status: number,
  ) {
    super(message);
  }
}
