# services/ai/src

Local offline AI adapter layer.

Rules:
- No cloud SDK usage in runtime path.
- AI modules produce proposals only.
- Engine validates and commits canonical events.

## Current scaffold

- `interfaces.ts`: contracts for module input/output and orchestrator behavior.
- `local_orchestrator.ts`: offline stub modules for narration, adjudication, encounter, NPC dialogue, and summarization.
- `index.ts`: exports for local integration.
