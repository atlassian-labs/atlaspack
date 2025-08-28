export const a = () => 'a';
export const b = () => 'b';
export const c = () => 'c';

const obj = {
  main: {
    d: 'd',
  },
};

export const { main: { d } } = obj;