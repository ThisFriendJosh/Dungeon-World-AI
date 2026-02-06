export type AiModuleName =
  | "narration"
  | "adjudication"
  | "encounter"
  | "npc_dialogue"
  | "summarizer";

export type AiContext = {
  campaignId: string;
  characterIds: string[];
  latestEventSeq?: number;
  summary?: string;
};

export type AiProposal = {
  module: AiModuleName;
  reason: string;
  proposedEvents: Array<{
    eventType: string;
    payload: Record<string, unknown>;
  }>;
};

export type AiModule = {
  name: AiModuleName;
  propose(input: {
    prompt: string;
    context: AiContext;
  }): Promise<AiProposal>;
};

export type AiOrchestrator = {
  runModule(module: AiModuleName, input: { prompt: string; context: AiContext }): Promise<AiProposal>;
};
