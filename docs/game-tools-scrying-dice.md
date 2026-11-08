# Game tools, Scrying Pane, and 3D dice

These campaign tabs appear first in the requested priority order: **Game Timers & Calculators**, **Scrying Pane**, **3D Dice**. The existing Dungeon Master, character, tabletop, art, rules, and Compendium views follow them.

## 1. Game Timers & Calculators

Each campaign has a saved game clock and up to 20 named countdowns or stopwatches. Choose a real clock for player turns and sessions, or a game clock for spell durations, torches, and rests. Presets fill in editable values; Add creates a paused timer, then Start begins it. Pause, Reset, and confirmed Remove are available per timer.

Running real timers use a saved wall-clock anchor, including while the application is closed. Game timers tick only when the user advances game time. Advance one round (six seconds), one minute, ten minutes, one hour, eight hours, or a custom number of seconds up to one day per action. Timer transitions and game-time changes are transactional, revision checked, and recorded once per action. No per-second database writes are made. Expired timers show Complete; no sound, external notification, or sheet mutation is triggered.

**Adjust game time** provides Remove time (seconds, rounds, minutes, hours, or days) and Set game time (days/hours/minutes/seconds). Enter zeros to reset the elapsed clock. Use current clock restores the saved value into the edit fields. These are clock corrections: they retain timer values and completed/paused states; Advance remains the action that ticks game timers. Corrections are bounded between zero and 3650 days, reject subtraction below zero, and save a before/after audit entry atomically with the changed clock.

Calculator tools include bounded arithmetic with parentheses, a rules-profile calculator for ability/proficiency/spell-save/concentration/exhaustion values, and damage/healing previews with temporary HP, immunity, resistance, vulnerability, and healing caps. A saved character can supply Strength, level, HP and speed as starting values. Calculations are previews; apply chosen results to the character sheet separately. Spell effects, concentration, death saves, rests and resource recovery remain manual. The DM receives a snapshot of the clocks explicitly labeled as timekeeping.

## 2. Scrying Pane

The browser uses **Chromium Embedded Framework through CefSharp**, hosted as a native child pane inside the Tauri desktop. The existing game interface continues to use its current renderer. Browser pages receive no JavaScript bindings to the game and cannot call Tauri commands. Browser commands use bounded local pipes, not a local HTTP bridge. Only explicit HTTP/HTTPS navigation is accepted; files, executable URLs, embedded credentials, and the application's internal origin are rejected. Ordinary in-page content, frames, and the browser's PDF viewer use CEF's own browser security rules.

Use the address bar, Back, Forward, Reload and Stop. User-requested popups open in the same pane. Bookmarks are saved with the active campaign; duplicate addresses update their titles. Website state and cookies use a separate local browser profile under the application data folder. Switching tabs hides the pane; **Unload pane** closes it to release browser memory. Closing the desktop also closes its owned browser process tree. Browsing requires the user's internet connection; the DM and local game tools retain their offline operation.

Run `Setup-Scrying-Pane.cmd` once to install the separate browser runtime. The setup uses Python 3.11+, Windows' existing .NET Framework compiler, and official pinned NuGet packages:

| Package | Version |
| --- | --- |
| CefSharp.WinForms | 156.0.150 |
| CefSharp.Common | 156.0.150 |
| chromiumembeddedframework.runtime.win-x64 | 156.0.15 |
| System.Runtime.InteropServices.RuntimeInformation | 4.3.0 |

NuGet catalog SHA512 hashes verify the downloads before extraction. Compressed and plain metadata are supported. Verified cached packages can rebuild the host without a new download. Package paths are checked before extraction. Builds are versioned by host-source fingerprint under `build/scrying-cef`; `current.json` selects the completed host. Model weights, browser binaries, profiles and package caches stay out of source control. Package notices and Chromium credits are retained with the host.

Set `DWA_SETUP_PYTHON_PATH` for another Python installation or `DWA_CEF_BROWSER_PATH` for an explicitly prepared ScryingPane.exe. Windows x64 is the implemented embedding target; other platforms report an unavailable runtime. An installer containing the browser runtime and clean-machine prerequisite checks remain distribution work. The helper's `--smoke-test --profile <absolute-folder>` option loads about:blank and emits its actual CEF and Chromium versions without requiring an internet request.

## 3. 3D Dice

The dice tray and the tabletop's embedded dice panel show shaded, rotating 3D polyhedra. d4, d6, d8, d10, d12 and d20 have their corresponding convex meshes. d100 uses two d10s; 00 and 0 represent 100. Custom die sizes retain their saved result cards. The tray shows up to 24 dice, or eight in a narrow panel; the full roll remains in history.

Results are generated and saved by the existing engine before animation. The animation only presents those faces; it does not roll, alter, retry or recalculate them. Discarded advantage/disadvantage dice remain visible and marked. Negative dice keep their signs. Replay visual replays the saved result. Animation can be disabled, dice color can be changed, and the system's reduced-motion preference is honored. These presentation preferences are stored on the device. The renderer has no external assets or runtime downloads and does not require WebGL.

## Verification

Engine tests cover timer persistence, wall-clock pause/resume, game-time advances, stale and foreign edits, transaction rollback, arithmetic validation, damage rounding, bookmark boundaries and all previous saves. UI tests cover tab order, clock values, escaping, campaign isolation, URL validation, convex meshes, face orientation, percentile representation, and unchanged saved dice. CEF was compiled from the pinned packages and its actual renderer completed an about:blank smoke test. Full user website/login coverage, multi-monitor/DPI behavior, and live in-pane navigation remain host acceptance checks.
