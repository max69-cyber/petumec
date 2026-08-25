// Геометрия атласа формата codex pet.

import type { AtlasLayout } from "../types";

// Колонок всегда восемь, в обеих версиях формата.
export const CODEX_COLS = 8;

// Пропорции эталонного v2-листа: 8 колонок по 192px = 1536 в ширину, и
// 11 строк по 208px = 2288 в высоту.
const V2_WIDTH = 1536;
const V2_HEIGHT = 2288;

// Число строк выводится из пропорции листа
export function codexRowCount(width: number, height: number): 9 | 11 {
  return height * V2_WIDTH >= width * V2_HEIGHT ? 11 : 9;
}

export function codexLayout(width: number, height: number): AtlasLayout {
  const rows = codexRowCount(width, height);
  return {
    cols: CODEX_COLS,
    rows,
    frameW: Math.floor(width / CODEX_COLS),
    frameH: Math.floor(height / rows),
  };
}
