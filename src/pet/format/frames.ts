// Правила сборки таблиц кадров — общие для любого формата

import type { FrameSpec } from "./types";

/// Взял идею из petdex — для анимаций все кадры кроме последнего будут обрабатываться одно время,
/// а последний другое, как правило удлиненный
export function uniform(count: number, dur: number, last: number): FrameSpec[] {
  return Array.from({ length: count }, (_, i) => ({
    col: i,
    durMs: i === count - 1 ? last : dur,
  }));
}
