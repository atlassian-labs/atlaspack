export const token = (path) => {
  const tokens = {
    'color.border.discovery': '#6554C0',
    'space.0': '0px'
  };
  return tokens[path] || `var(--ds-${path.replace(/\./g, '-')})`;
};