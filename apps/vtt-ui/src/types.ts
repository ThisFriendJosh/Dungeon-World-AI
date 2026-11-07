export type CampaignSummary = {
  id: string; name: string; slug: string; createdAt: string; updatedAt: string;
};
export type CharacterSummary = {
  id: string; campaignId: string; name: string; className: string; level: number;
  statsJson: string; inventoryJson: string; conditionsJson: string; progressionJson: string;
  createdAt: string; updatedAt: string;
};
export type CampaignEvent = {
  seq: number; eventType: string; payload: Record<string, unknown>; createdAt: string;
};
export type Saved<T> = { value: T; warning: string | null };
