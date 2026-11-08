export const WORKSPACE_TABS=[
  ["tools","Game Timers & Calculators"],["scrying","Scrying Pane"],["dice","3D Dice"],
  ["dm","Dungeon Master"],["character","Characters"],["tabletop","Tabletop"],
  ["art","Art & codex"],["rules","Rules"],["compendium","Compendium"]
] as const;
export type WorkspaceView=typeof WORKSPACE_TABS[number][0];
