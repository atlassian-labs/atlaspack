# @atlaspack/analytics

This package propagates runtime analytics events as custom events to the client `window` object to aid in tracking runtime metrics and experiments.

This is designed such that the consumer can subscribe to events and forward them to their internal analytics service.

## Note

This is for to aid in experiments so dispatch sparingly and clean up events when experiments are complete.

## Usage

### Within Atlaspack

```typescript
import {sendOperationalEvent} from '@atlaspack/analytics';

sendOperationalEvent({
  // Required
  action: 'experimentName',
  // Optional
  attributes: {arbitrary: 'values'},
  // Optional
  tags: ['arbitrary', 'values'],
});
```

### From consumer

```html
<html>
  <body>
    <!-- Client's analytics service -->
    <script src="/analytics-client.js"></script>

    <!-- 
    Capture runtime events from Atlaspack and
    forward them to client's analytics service
  -->
    <script>
      window.addEventListener('atlaspack:analytics', ({detail}) => {
        // Analytics event is available under `detail`
        console.log(detail);

        // Forward to the respective analytics service
        window.analyticsClient.dispatchEvent(detail.action, detail);
      });
    </script>

    <!-- Run the bundle -->
    <script src="./bundle.js" type="module"></script>
  </body>
</html>
```
