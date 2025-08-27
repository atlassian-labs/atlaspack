import { foo } from './foo';

alert('Hello ' + foo);

import('./dependency').then(({ default: depExport }) => {
  alert('Dependency exported: ' + depExport);
});
