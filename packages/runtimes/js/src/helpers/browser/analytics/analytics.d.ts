export type AtlaspackAnalyticsEvent = {
  kind?: string;
  action: string;
  attributes?: {[key: string]: string | number | boolean};
  tags?: string[];
};

export function sendAnalyticsEvent(event: AtlaspackAnalyticsEvent): void;
