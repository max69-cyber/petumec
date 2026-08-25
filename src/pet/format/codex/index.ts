import type { SpriteFormat, StateDef } from "../types";
import { codexLayout } from "./layout";
import { CODEX_STATE_DEFS, CODEX_STATES, type CodexState } from "./states";

export type { CodexState };

export const codexPetFormat: SpriteFormat<CodexState> = {
  id: "codex-pet",
  defaultState: "idle",
  states: CODEX_STATES,
  layoutFor: codexLayout,
  stateDef(state: CodexState): StateDef {
    return CODEX_STATE_DEFS[state];
  },
};
