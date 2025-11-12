import * as React from 'react';
import { ax, ix } from "@compiled/react/runtime";
import { jsx } from "react/jsx-runtime";
const underline = {
    '&::after': {
        position: 'absolute',
        borderRadius: '1px'
    }
};
const pressed = {
    '&::after': {
        ...underline['&::after'],
        backgroundColor: 'red'
    }
};
const styles = null;
export const Component = ()=>jsx("div", {
        className: ax([
            "_19v6t94y _5or0t94y _qrdit94y _174kt94y _ee5ostnw _1tfxstnw _ixiwstnw _1uf0stnw _gcm15scu"
        ])
    });
