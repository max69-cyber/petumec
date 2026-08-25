// Контракт спрайтового формата

/// Один кадр анимации: колонка атласа и сколько кадр держится
export type FrameSpec = {
  col: number;
  durMs: number;
};

/// Одно состояние (анимация): строка атласа и её кадры по порядку
export type StateDef = {
  row: number;
  frames: FrameSpec[];
};

/// Сетка атласа и размер кадра в пикселях исходника
export type AtlasLayout = {
  cols: number;
  rows: number;
  frameW: number;
  frameH: number;
};

/// Дженерик по строковому union состояний: у каждого формата свой набор
/// имён (возможно в будущем)
export interface SpriteFormat<State extends string = string> {
  readonly id: string;
  readonly states: readonly State[];

  // Начальное состояние, с которого начинается проигрывание
  readonly defaultState: State;

  // Разбор геометрии по фактическим размерам картинки. Метод, а не
  // константа, ибо число строк не всегда записано в метаданных и может
  // выводиться из самого листа (как это сейчас в codex формате)
  layoutFor(width: number, height: number): AtlasLayout;

  // Строки из таблиц анимаций и тайминги определенных кадров.
  stateDef(state: State): StateDef;
}
