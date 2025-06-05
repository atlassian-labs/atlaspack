import pino from 'pino';
import pinoPretty from 'pino-pretty';

export const logger = pino(
  {
    level: 'info',
  },
  pinoPretty(),
);
