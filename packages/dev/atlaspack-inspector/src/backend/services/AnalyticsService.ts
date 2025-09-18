import {randomUUID} from 'crypto';
import {logger} from '../config/logger';

export type AnalyticsServiceAvailableResponse =
  | {
      available: true;
      name: string;
      version: string;
    }
  | {
      available: false;
    };

export interface AnalyticsEvent {
  id?: string;
  cwd?: string;
  data: {
    name?: string;
    category?: string;
    status?: 'success' | 'failure';
    action: string;
    startTimestamp?: Date;
    product?: string;
  };
}

/**
 * Reports analytics events to a daemon server running at port 16621.
 *
 * The application will automatically check if the service is available and will not report
 * anything if it is not.
 */
export class AnalyticsService {
  private readonly daemonUrl: string = 'http://localhost:16621';
  private isAvailable: boolean | null = null;

  /**
   * Return the analytics service status.
   */
  public async checkAvailable(): Promise<AnalyticsServiceAvailableResponse> {
    const response = await fetch(`${this.daemonUrl}/version`);
    if (!response.ok) {
      const status = response.status;
      const body = await response.json();
      logger.warn(
        {body, status},
        'Failed to check analytics service availability',
      );
      this.isAvailable = false;
      return {available: false};
    }

    const data: any = await response.json();

    this.isAvailable = true;

    return {
      available: data.version !== 'unknown',
      name: data.name,
      version: data.version,
    };
  }

  /**
   * Record an analytics event.
   */
  async sendEvent(event: AnalyticsEvent): Promise<void> {
    try {
      if (this.isAvailable == null) {
        const available = await this.checkAvailable();
        logger.debug(
          available,
          `Analytics service is ${available.available ? 'available' : 'not available'}`,
        );
      }
      if (!this.isAvailable) {
        return;
      }

      const body = {
        id: `atlaspack-inspector-${randomUUID()}`,
        cwd: process.cwd(),
        type: 'DISCRETE_EVENT',
        ...event,
        data: {
          startTimestamp: new Date().toISOString(),
          status: 'success',
          product: 'atlaspack-inspector',
          category: 'atlaspack-internal',
          name: 'atlaspack-inspector',
          ...event.data,
        },
      };
      logger.debug({event: body}, 'Tracking analytics event');
      const response = await fetch(`${this.daemonUrl}/events`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(body),
      });

      if (!response.ok) {
        const status = response.status;
        const body = await response.json();
        logger.warn({body, status}, 'Failed to send analytics event, skipping');
        return;
      }

      const json = await response.json();
      logger.debug({json}, 'Analytics event sent');
    } catch (err) {
      logger.warn({err}, 'Failed to send analytics event');
    }
  }
}

export const analyticsService = new AnalyticsService();
