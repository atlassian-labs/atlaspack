export class HTTPError extends Error {
  constructor(message: string, public status: number) {
    super(message);
  }
}
