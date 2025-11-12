import { styled } from '@compiled/react';

export const Component = styled.div({
  backgroundColor: (props) => {
    // Nested ternary operator is needed, otherwise there's a bug that the first token is applied regardless of condition
    // eslint-disable-next-line no-nested-ternary
    return props.isRowSelected
      ? "var(--ds-background-selected, #E9F2FF)"
      : props.formatRuleBackgroundColor
        ? props.formatRuleBackgroundColor
        : "var(--ds-background-neutral-subtle, #00000000)";
  },
});
