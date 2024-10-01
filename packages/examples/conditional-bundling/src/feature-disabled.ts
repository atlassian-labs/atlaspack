export default () => 'The feature is DISABLED';
// @ts-expect-error - TS7006 - Parameter 'a' implicitly has an 'any' type. | TS7006 - Parameter 'b' implicitly has an 'any' type.
export const add = (a, b) => a + b;
