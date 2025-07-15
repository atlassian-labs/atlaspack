export class AlreadyDisposedError extends Error {
  constructor(message?: string) {
    super(message);
    this.name = 'AlreadyDisposedError';
  }
}
