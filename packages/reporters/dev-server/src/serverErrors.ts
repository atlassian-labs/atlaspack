export type ServerError = Error & {
  code: string;
};

const serverErrorList = {
  EACCES: "You don't have access to bind the server to port {port}.",
  EADDRINUSE: 'There is already a process listening on port {port}.',
} as const;

export default function serverErrors(err: ServerError, port: number): string {
  let desc = `Error: ${
    err.code
  } occurred while setting up server on port ${port.toString()}.`;

  // @ts-expect-error TS7053
  if (serverErrorList[err.code]) {
    // @ts-expect-error TS7053
    desc = serverErrorList[err.code].replace(/{port}/g, port);
  }

  return desc;
}
