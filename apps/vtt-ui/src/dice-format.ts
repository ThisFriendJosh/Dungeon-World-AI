import type { DiceRoll, DiceTerm } from "./dice-types";
export function describeTerm(term: DiceTerm): string {
  if (term.kind === "modifier") return (term.value >= 0 ? "+" : "") + term.value;
  const faces = term.rolls.map((face, index) => face + (term.kept[index] ? " kept" : " discarded")).join(", ");
  return (term.sign < 0 ? "-" : "") + term.count + "d" + term.sides + ": " + faces + " = " + term.subtotal;
}
export function describeRoll(roll: DiceRoll): string {
  const mode = roll.mode === "normal" ? "" : " (" + roll.mode + ")";
  return roll.expression + mode + " = " + roll.total;
}
