import { foo } from './foo';

alert('Hello ' + foo);

const depPromise = import('./dependency');

console.log('depPromise=', depPromise);

depPromise.then(({ default: depExport }) => {
  alert('Dependency exported: ' + depExport);
});
