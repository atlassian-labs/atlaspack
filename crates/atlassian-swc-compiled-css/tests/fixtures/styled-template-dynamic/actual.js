import { styled } from '@compiled/react';
import { jsx, jsxs } from "react/jsx-runtime";
const StyledButton = styled.button`
  background-color: ${(props)=>props.primary ? 'blue' : 'gray'};
  color: white;
  padding: 8px 16px;
  border: none;
  border-radius: 4px;
  font-size: 14px;
  
  &:hover {
    background-color: ${(props)=>props.primary ? 'darkblue' : 'darkgray'};
  }
  
  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
`;
function App() {
    return jsxs("div", {
        children: [
            jsx(StyledButton, {
                primary: true,
                children: "Primary Button"
            }),
            jsx(StyledButton, {
                children: "Secondary Button"
            })
        ]
    });
}
