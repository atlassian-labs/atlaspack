import { forwardRef } from "react";
import * as React from "react";
import { ax, ix } from "@compiled/react/runtime";
const tabStyles = null;
export const Component = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return <C {...__cmplp} style={{
        ...__cmpls,
        "--_b3nd5v": ix(__cmplp.isDraggable ? 'none' : 'flex'),
        "--_11m4mys": ix(__cmplp.isDraggable ? 'flex' : 'none')
    }} ref={__cmplr} className={ax([
        "_15y3r8wq _oyoh2fjc",
        __cmplp.isDragging && "_tzy41kuy",
        __cmplp.className
    ])}/>;
});
if (process.env.NODE_ENV !== "production") {
    Component.displayName = "Component";
}
