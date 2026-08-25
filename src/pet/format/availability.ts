// Какие состояния реально доступны на конкретном листе.
//
// Набор состояний — свойство не формата, а загруженного файла. Формат codex
// знает про 11 строк, но лист v1 несёт только 9: состояния из строк 9 и 10
// на нём просто нечем нарисовать.
//
// Фильтр идёт по номеру строки, не по позиции в format.states — у codex эти
// порядки совпадают случайно. И это проверка границ, а не содержимого: лист
// на 11 строк с пустыми строками 9-10 её пройдёт.

import type { AtlasLayout, SpriteFormat } from "./types";

export function isAvailable<S extends string>(
  format: SpriteFormat<S>,
  state: S,
  layout: AtlasLayout,
): boolean {
  return format.stateDef(state).row < layout.rows;
}

export function availableStates<S extends string>(
  format: SpriteFormat<S>,
  layout: AtlasLayout,
): readonly S[] {
  return format.states.filter((state) => isAvailable(format, state, layout));
}
