// Таблица анимаций формата codex pet.
// Строка атласа = состояние, колонка = кадр внутри него.

import { uniform } from "../frames";
import type { FrameSpec, StateDef } from "../types";

// Пока делаю по референсу petdex, так что работаем с таблицей codex:v1
// где нет анимаций осмотра вокруг
export const CODEX_STATES = [
  "idle",
  "running-right",
  "running-left",
  "waving",
  "jumping",
  "failed",
  "waiting",
  "running",
  "review",
] as const;

export type CodexState = (typeof CODEX_STATES)[number];

// Тайминги для idle пока тоже из petdex
const IDLE_FRAMES: FrameSpec[] = [
  { col: 0, durMs: 280 },
  { col: 1, durMs: 110 },
  { col: 2, durMs: 110 },
  { col: 3, durMs: 140 },
  { col: 4, durMs: 140 },
  { col: 5, durMs: 320 },
];

// Остальное все прогоним через хелпер — пока тоже значения из petdex
export const CODEX_STATE_DEFS: Record<CodexState, StateDef> = {
  idle: { row: 0, frames: IDLE_FRAMES },
  "running-right": { row: 1, frames: uniform(8, 120, 220) },
  "running-left": { row: 2, frames: uniform(8, 120, 220) },
  waving: { row: 3, frames: uniform(4, 140, 280) },
  jumping: { row: 4, frames: uniform(5, 140, 280) },
  failed: { row: 5, frames: uniform(8, 140, 240) },
  waiting: { row: 6, frames: uniform(6, 150, 260) },
  running: { row: 7, frames: uniform(6, 120, 220) },
  review: { row: 8, frames: uniform(6, 150, 280) },
};
