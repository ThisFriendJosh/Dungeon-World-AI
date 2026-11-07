# Windows native verification

Verified 2026-11-07 against this update's application code.

The Windows development executable was built and exercised through its actual WebView2 UI and Rust command bridge. No browser fixture or fake backend was active. The run used a stable copy of an existing campaign, a dedicated DWA_DATA_DIR, and separate WebView2 profiles.

## Results

| Check | Result |
| --- | --- |
| Startup and existing campaign load | Passed through native IPC |
| Existing character | Name, level, stats, inventory, conditions, progression, and timestamps preserved |
| Legacy scene notes | Both GM cards and their content preserved |
| Edition migration | Existing campaign opened as manual; choosing revised rules preserved the sheet |
| Maps and pieces | Supplied background and portrait selection, five-foot nudge, notes, state labels, and light radii saved |
| Fog and visibility | Brush marks saved; player map omitted the GM-only piece |
| Local artwork | Background and sprite imports persisted as separate image records |
| Scenes | Two scenes saved and switched successfully |
| Dice | Actual engine advantage roll saved; unsaved map edits remained intact |
| Rules | Saved profile, concentration and exhaustion calculations, offline SRD search, and attribution view passed |
| Compendium | All six collections imported through native file selection: 33,983 entries / 193 source-book labels |
| Source filters | Revised spell lookup retained source book, publisher, and year tag |
| Reopen | Fresh process and fresh WebView2 profile restored the active scene, images, pieces, fog/light, cards, dice, rules, and all collections |
| Original campaign | Database hash unchanged after testing |
| Native page errors | None |

The existing character was compared before and after reference imports, profile changes, and reopening. Its stored values were unchanged. Imported images were loaded from SQLite into a fresh webview, confirming that browser memory or browser storage was not responsible for persistence.

All verification processes and the temporary development server were stopped afterward. Private campaign copies, imported collections, and screenshots are excluded from source control.

## Reproduction boundary

Use a copied campaign and a dedicated DWA_DATA_DIR for destructive or persistence checks. The app's normal development command remains npm run dev, with npm.cmd in Windows PowerShell when needed.

The automated native run used a separate development port and WebView2 profile/debugging settings. Those settings were supplied only to the verification process; application configuration and registry settings were unchanged. Microsoft documents the WebView2 environment overrides at:
https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/webview2-idl

This verifies the Windows development app. NSIS installer launch, production-package installation, and Linux packaging remain separate release checks. The 45 Rust and 19 UI tests, browser scenarios, TypeScript, production UI bundle, desktop compilation, catalog consistency, and offline policy checks also pass.
