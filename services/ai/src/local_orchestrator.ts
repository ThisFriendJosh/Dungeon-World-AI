import type { AiModule, AiModuleName, AiOrchestrator, AiProposal, AiContext } from "./interfaces";

class StubModule implements AiModule {
  constructor(public readonly name: AiModuleName) {}

  async propose(input: { prompt: string; context: AiContext }): Promise<AiProposal> {
    return {
      module: this.name,
      reason: "offline-stub",
      proposedEvents: [
        {
          eventType: `ai.${this.name}.proposed`,
          payload: {
            prompt: input.prompt,
            campaignId: input.context.campaignId,
            characterIds: input.context.characterIds,
            latestEventSeq: input.context.latestEventSeq ?? null
          }
        }
      ]
    };
  }
}

const modules: Record<AiModuleName, AiModule> = {
  narration: new StubModule("narration"),
  adjudication: new StubModule("adjudication"),
  encounter: new StubModule("encounter"),
  npc_dialogue: new StubModule("npc_dialogue"),
  summarizer: new StubModule("summarizer")
};

export const createLocalOrchestrator = (): AiOrchestrator => ({
  async runModule(module, input) {
    return modules[module].propose(input);
  }
});
