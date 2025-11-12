import { jsx } from "react/jsx-runtime";
const styles = {
    container: "_2rko1l7b _1reo15vq _18m915vq _1e0c116y _vchhusvi _kqswpfqs _1kz6184x _bozg1y44 _y4ti1y44",
    containerSubtle: "_1cwg1n1a",
    text: "_1reo15vq _18m915vq _ect49sn6 _1wyb1skh _zg8l4jg8 _k48p8n31 _vwz47vkz _1bto1l2s _1p1dangw _o5721q9c",
    customLetterspacing: "_1dyzw1qx",
    bgBoldDefault: "_bfhk1fkg",
    bgBoldInprogress: "_bfhk1ymo",
    bgBoldMoved: "_bfhkxmjf",
    bgBoldNew: "_bfhkshej",
    bgBoldRemoved: "_bfhk1366",
    bgBoldSuccess: "_bfhk3uhp",
    bgSubtleDefault: "_bfhk1hxd",
    bgSubtleInprogress: "_bfhk1hxd",
    bgSubtleMoved: "_bfhk1hxd",
    bgSubtleNew: "_bfhk1hxd",
    bgSubtleRemoved: "_bfhk1hxd",
    bgSubtleSuccess: "_bfhk1hxd",
    borderSubtleDefault: "_19it14mp",
    borderSubtleInprogress: "_19it1cy7",
    borderSubtleMoved: "_19itzi1n",
    borderSubtleNew: "_19it1apr",
    borderSubtleRemoved: "_19itoa5t",
    borderSubtleSuccess: "_19it1am1",
    textSubtle: "_syazalr3",
    textBold: "_syazwwip"
};
function Lozenge({ children, isBold = false, appearance = 'default' }) {
    const appearanceStyle = isBold ? 'Bold' : 'Subtle';
    const bgClass = `bg${appearanceStyle}${appearance.charAt(0).toUpperCase() + appearance.slice(1)}`;
    const textClass = `text${appearanceStyle}`;
    return jsx("span", {
        className: styles.container(),
        children: jsx("span", {
            className: styles[bgClass](),
            children: jsx("span", {
                className: styles.text(),
                children: jsx("span", {
                    className: styles.customLetterspacing(),
                    children: jsx("span", {
                        className: styles[textClass](),
                        children: children
                    })
                })
            })
        })
    });
}
export default Lozenge;
