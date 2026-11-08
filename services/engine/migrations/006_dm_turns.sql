CREATE TABLE IF NOT EXISTS dm_turns (
  id TEXT PRIMARY KEY,
  campaign_id TEXT NOT NULL REFERENCES campaigns(id),
  request_id TEXT NOT NULL,
  actor_id TEXT,
  player_text TEXT NOT NULL,
  proposal_json TEXT NOT NULL,
  model TEXT NOT NULL,
  prompt_version TEXT NOT NULL,
  based_on_seq INTEGER NOT NULL,
  event_seq INTEGER NOT NULL REFERENCES events(seq),
  created_at TEXT NOT NULL,
  UNIQUE(campaign_id, request_id)
);
CREATE INDEX IF NOT EXISTS idx_dm_turns_campaign ON dm_turns(campaign_id, event_seq);
