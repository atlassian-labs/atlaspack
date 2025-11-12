import { cx } from '@atlaskit/css';
import { jsx } from "react/jsx-runtime";
const styles = {
    base: "_syaz1wbm _ca0qdbr4",
    extra: "_otyru43s"
};
export const Component = ({ showExtra })=>jsx("div", {
        xcss: cx(styles.base, showExtra ? styles.extra : null),
        children: "content"
    });
