# AI Dungeon Master — first local integration

This development milestone connects a campaign-aware local narrator to the existing desktop app. It implements the integration code, persistence, and interface; live model/desktop verification and complete offline packaging remain release gates.

Workspace update: the current working tree adds local story/reference search, typed DM read/calculation tools, GM table knowledge, PNG/SVG token assets and reversible history cleanup. See the [DM workspace guide](dm-workspace.md) for current capabilities and limits; the baseline descriptions below record the initial narrator milestone. The user's subsequent screenshot confirms one saved local reply in that initial build. It does not establish that the newer read-tool loop or a clean offline installer has passed qualification.

## Try the development build

From the repository root:

```powershell
npm.cmd run dev:dm
```

The development launcher reuses the existing Unsloth llama.cpp executable and the cached `Qwen3-8B-Q4_K_M.gguf` when present. It starts its own model process; the Unsloth application/server does not need to be running. It never downloads a model. Node, Rust, and the existing Tauri development prerequisites are still required for this development command.

For a different local installation, set file paths in the launching shell:

```powershell
$env:LOCAL_LLM_SERVER_PATH = 'C:\LocalAI\llama-server.exe'
$env:LOCAL_LLM_MODEL_PATH = 'C:\LocalAI\models\dungeon-master.gguf'
npm.cmd run dev:dm
```

The runtime executable must keep its matching native libraries alongside it. The app passes arguments for a text-only llama.cpp server with JSON-schema responses, an 8,192-token context, one slot, and thinking disabled. The installed development build exposes the required flags; other runtime/model combinations require separate qualification. Set `DWA_DM_GPU_LAYERS=0` to select CPU mode; CPU playability has not been benchmarked.

Open/create a campaign, record actual character values, and choose **Dungeon Master**. Select a character or the party, describe an action, and send it. The model loads on the first turn. Accepted turns save automatically; saving does not end the session. Optional next-action buttons fill the composer so the player chooses what to send.

For a check, use the **Dice** tab to make and save the actual roll, return to Dungeon Master, and choose **Use latest saved roll**. The button references the canonical roll ID. The engine supplies the six most recent recorded rolls to the narrator. The formula, bonus, DC, and ruling remain table decisions in this version; entering a result in chat does not create an engine roll.

## What is implemented

| Layer | Behavior |
| --- | --- |
| Desktop interface | Campaign conversation, actor selector, action composer, suggested actions, progress, stop, error recovery, and saved timestamps |
| Context | Campaign identity/revision, up to 12 characters, selected edition/house rules, public scene cards, six real dice rolls, and up to 12 recent conversation turns |
| Local runtime | Rust-owned process, numeric IPv4 loopback socket, per-launch authentication, local model file, startup/request timeouts, token counting, response bounds, cancellation, normal-exit cleanup |
| Proposal validation | Completed JSON reply with only schema version, narration, and choices; no arbitrary operations or events |
| Persistence | Campaign-scoped SQLite turns and their events in one transaction, saved request IDs for duplicate suppression, event-log recovery |
| Concurrency | Recheck event sequence before commit; discard replies generated against changed campaign state |
| Rendering | Player/model text is escaped; generated text is never inserted as executable HTML |

The transport has no configurable server URL, DNS lookup, proxy handling, redirect following, cloud SDK, or cloud fallback. It uses a small bounded HTTP/1.1 JSON transport directly to the port allocated for the app-owned child. The existing offline source policy stays unchanged. The app process and sidecar exchange text; the model has no database, filesystem, shell, or game-operation interface.

The engine omits private scene cards, scene/GM notes, token notes, raw scene events, and hidden/fogged entities from narration context. Public cards are available to the narrator; put secrets in private cards. Hidden-information behavior in generated prose still needs model evaluation.

## Current limits

- HP, inventory, conditions, treasure, spell resources, combat, and tabletop changes are manual. A narrator cannot perform these writes. Prose can still contain incorrect rulings and requires review.
- The model sees the rules profile and house rules, rather than automatic full rulebook retrieval. No published rule/page claim is validated by this milestone.
- All accepted turns persist, but the interface shows the latest 50 and inference uses a recent window. Older turns are removed from the prompt as needed to reserve output space. There is no durable summary or long-term fact retrieval yet.
- Recorded sheet values can still be placeholders; a class/level alone does not establish a proficiency, feature, attack bonus, or damage expression.
- Replies arrive after validation and saving. Streaming narration is not implemented.
- Stopping generation preserves the unsent draft and committed campaign; it unloads the app-owned model. The next turn reloads it. A cancellation received after a commit cannot undo that saved turn.
- The distribution does not yet bundle/pin a runtime/model manifest or ship the offline WebView2 prerequisite. This is not the finished standalone installer.
- Optional GPT connectivity remains a separate future adapter. This milestone has no online model provider.

## Verification

```powershell
npm.cmd run check
npm.cmd run test:ui
npm.cmd run build:ui
cargo test --offline --manifest-path services/engine/Cargo.toml
cargo test --offline --manifest-path services/ai/Cargo.toml --all-targets
cargo check --offline --manifest-path apps/desktop/src-tauri/Cargo.toml
```

The loopback socket integration test is opt-in because restricted command environments can disallow local TCP connections:

```powershell
cargo test --offline --manifest-path services/ai/Cargo.toml --lib -- --ignored
```

For a real model check, set the two local file paths above and run:

```powershell
cargo run --offline --manifest-path services/ai/Cargo.toml --example local_dm_smoke
```

That example creates a fresh isolated test save, asks for three real replies, records an engine roll, checks conversation/roll survival after reopening, checks that the sheet was unchanged, and shuts down the model. It prints the test save path and reply timings for review. Continuity/agency/rule correctness require human review of those replies; the example does not assert that a model is a qualified DM.

Implementation-session results: 51 engine tests, 4 AI unit tests, and 23 UI tests passed (78 total). Type checking, the unchanged offline source policy, the UI production build, and the native Rust compile check passed. The opt-in socket test failed with Windows error 10013 (socket access forbidden by permissions), and desktop automation reported `failed to launch codex app-server: program not found`. Therefore the live model run, native UI behavior, and clean offline installer are **unverified**, independently of the passing compile/unit/render checks. The socket test remains explicitly opt-in and must pass on the host before release.

Next: run the real-model/native checks on the host, add reviewed persistent facts/summaries and a validated pending-check protocol, then qualify a complete offline package against the [distribution design](offline-dm-design.md).
