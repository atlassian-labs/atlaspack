import {colorPalette} from './colorPalette';

export function getRandomDarkerColor(name: string): {
  familyName: string;
  family: string[];
  shade: string;
} {
  const hash = name.split('').reduce((acc, char) => {
    return acc + char.charCodeAt(0);
  }, 0);
  const randomIndex = hash % Object.keys(colorPalette).length;
  const randomFamily = Object.keys(colorPalette)[
    randomIndex
  ] as keyof typeof colorPalette;
  const darkerShades = randomFamily.slice(6);
  const randomShade =
    darkerShades[Math.floor(Math.random() * darkerShades.length)];

  return {
    familyName: randomFamily,
    family: colorPalette[randomFamily],
    shade: randomShade,
  };
}
