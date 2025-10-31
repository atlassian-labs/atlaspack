import { styled } from '@compiled/react';
import { jsx } from "react/jsx-runtime";
const gridSize = 8;
const Container = styled.div(({ hideDropdownLabel })=>({
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        minHeight: `${gridSize * (hideDropdownLabel ? 14 : 17)}px`,
        overflow: 'hidden'
    }));
export const Component = ({ hideDropdownLabel })=>jsx(Container, {
        hideDropdownLabel: hideDropdownLabel
    });
