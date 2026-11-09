export type DmProposal = {schemaVersion:1;narration:string;choices:string[]};
export type DmTurn = {
  id:string;campaignId:string;requestId:string;actorId:string|null;playerText:string;reply:DmProposal;
  model:string;promptVersion:string;basedOnSeq:number;eventSeq:number;createdAt:string;
};
export type DmHistory = {turns:DmTurn[];totalTurns:number;latestEventSeq:number};
export type DmRuntimeStatus = {configured:boolean;running:boolean;busy:boolean;model:string;message:string;progress?:{phase:string;elapsedSeconds:number;promptTokens:number|null;processedTokens:number|null;generatedTokens:number|null}|null};
