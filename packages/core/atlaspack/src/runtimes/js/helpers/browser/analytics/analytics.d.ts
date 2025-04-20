export type AtlaspackAnalyticsEvent = {
  action: string;
  attributes?: {[key: string]: string | number | boolean};
};

export function sendAnalyticsEvent(event: AtlaspackAnalyticsEvent): void;
