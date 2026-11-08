# services/ai

The `dwa-local-dm` Rust crate runs the first local Dungeon Master narrator. The desktop app owns the llama.cpp process, builds context through the campaign engine, validates the JSON reply, and commits the player action and narration together. It supports narration and NPC dialogue with manual mechanical resolution.

Run `npm run dev:dm` from the repository root to reuse an already installed development runtime/model. Explicit file overrides are `LOCAL_LLM_SERVER_PATH` and `LOCAL_LLM_MODEL_PATH`. No Unsloth server, cloud API key, or model download is used by the app.

See [setup, scope, and verification](../../docs/ai-dungeon-master.md). The older TypeScript interfaces and `local_orchestrator.ts` remain placeholders for future adjudication, encounters, and summarization; the desktop narrator uses the Rust crate directly. Do not route production turns through the stub.

Runtime requirement: **no cloud calls**.

The native narrator now supports bounded read-only tools for local references, saved scenes/tokens, character sections, grid distances and arithmetic. The engine retains state-write ownership. See the [DM workspace guide](../../docs/dm-workspace.md) for GM knowledge boundaries, manual mechanics, asset/history features, and current verification limits.
