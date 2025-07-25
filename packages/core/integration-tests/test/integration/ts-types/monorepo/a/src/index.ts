// @ts-expect-error TS2307
import { B } from "b";

export default function () {
	return new B();
}
