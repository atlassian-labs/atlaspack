import { cx } from '@atlaskit/css';
import { jsx } from "react/jsx-runtime";
const styles = {
    base: "_syaz1gjq _ca0qu2gc",
    extra: "_otyrpxbi"
};
export const Component = ({ showExtra })=>jsx("div", {
        xcss: cx(styles.base, showExtra ? styles.extra : null),
        children: "content"
    });
