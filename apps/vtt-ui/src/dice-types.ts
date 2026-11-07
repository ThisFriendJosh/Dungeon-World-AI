export type DiceMode = "normal" | "advantage" | "disadvantage";
export type DiceTerm =
  | { kind: "dice"; sign: number; count: number; sides: number; rolls: number[]; kept: boolean[]; subtotal: number }
  | { kind: "modifier"; value: number };
export type DiceRoll = {
  id: string; campaignId: string; characterId: string | null; actorName: string | null;
  label: string; expression: string; mode: DiceMode; terms: DiceTerm[];
  total: number; createdAt: string;
};
