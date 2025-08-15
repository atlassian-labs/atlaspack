import { symbol_7 } from './file_7';
import { symbol_10 } from './file_10';

export async function run() {
  let acc = 0;
  acc += await symbol_7();
  acc += await symbol_10();
  return acc;
}
