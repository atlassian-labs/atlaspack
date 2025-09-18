import pino from 'pino';
import pinoPretty from 'pino-pretty';

/**
 * Pino is used for logging.
 *
 * - https://github.com/pinojs/pino
 */
export const logger = pino(
  {
    level: process.env.PINO_LEVEL ?? 'info',
  },
  pinoPretty(),
);
