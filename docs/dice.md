# Local dice tray

Open a campaign and choose Dice. Select a die preset or enter a formula, choose Normal/Advantage/Disadvantage, optionally select a character and label, then choose Roll and save.

Selecting a character labels the result. Ability, proficiency, and other bonuses are entered in the formula. Results show each die face, kept/discarded d20s, signed modifiers, and the final total. Natural 1/20 outcomes and success, failure, damage, or critical hits remain table decisions.

## Supported notation

- 1d20+5
- d20+1d4+3
- 2d6+3
- 1d20-1d4+5
- 3d8-2

The parser supports adding and subtracting dice terms and integer modifiers. It ignores ASCII whitespace and accepts uppercase D. Multiplication, division, parentheses, exploding dice, Fate dice, and keep/drop notation are outside this first implementation.

Limits: 120 input bytes, 16 terms, 100 physical dice, 2-1000 sides, and an absolute modifier of at most 10000 per term.

Advantage/disadvantage requires exactly one positive 1d20 term. That term rolls twice and keeps the higher/lower face; ties keep one face. Other dice in the expression roll once. If advantage and disadvantage cancel, choose Normal.

## Persistence and randomness

The Rust engine accepts an expression, mode, optional character ID, and label. It obtains randomness from the operating system through getrandom and uses rejection sampling to avoid modulo bias. Clients supply no faces or totals.

A successful roll is an immutable dice.rolled event in the campaign database. Each click produces a new event. The Dice view returns the latest 50 rolls ordered by event sequence. Reopening a campaign reloads the recorded results without rerolling them. Actor names are captured at roll time, so later character renames preserve the historical label.

The existing transaction and recoverable JSONL mirror behavior applies. A failed database write does not display an unsaved roll as successful; a mirror failure warns that the database save succeeded. Rolls do not automatically alter HP, inventory, or game resources.

A future AI module must request a roll through the engine rather than proposing its own faces or totals.

## Dependency refresh

The UI pins Vite 6.4.5 and resolves esbuild 0.25.12. The user's terminal reported zero npm audit vulnerabilities after this targeted upgrade on 2026-11-07. The agent's environment cannot reach the npm audit endpoint; a fresh online audit remains part of release checks.

The updated dependency graph was used for TypeScript checking, adapter/display tests, and the Vite production bundle. Install-time script-review notices are separate from vulnerability reports.

## Checks

From the repository root:

```sh
npm run check
npm run test:ui
npm run test:engine
npm run check:rust
npm run build:ui
npm audit
```

Tests cover parser bounds, signed totals, advantage/disadvantage, ties, one-time extra dice, unbiased sampling, random-source failure, actor/campaign validation, saved result reloads, historical actor names, transaction failure, mirror recovery, and latest-50 ordering.

Manual native check: roll 1d20+5 with Advantage, confirm two faces and one kept result, close/reopen the app, and verify the same result returns in Dice history. Then roll 2d6+3 in Normal mode and confirm both faces contribute. Interactive dice behavior and installer packaging still require release verification.
