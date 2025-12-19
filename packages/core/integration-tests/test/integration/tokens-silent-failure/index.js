import {token} from '@atlaskit/tokens';

// This should cause an error: token() requires at least one argument
// But the error is silently ignored, so the build succeeds
const invalidToken = token();
console.log("INVALID TOKEN", invalidToken);

