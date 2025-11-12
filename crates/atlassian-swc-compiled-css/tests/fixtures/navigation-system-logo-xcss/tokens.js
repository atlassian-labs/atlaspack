export const token = (tokenPath, fallback) => {
  const tokens = {
    'radius.small': '3px',
    'color.background.neutral.subtle.hovered': '#F4F5F7',
    'color.background.neutral.subtle.pressed': '#EBECF0',
    'space.100': '8px',
  };
  return tokens[tokenPath] || fallback || tokenPath;
};