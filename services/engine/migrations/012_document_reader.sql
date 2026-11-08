CREATE TABLE IF NOT EXISTS knowledge_originals (
  source_id TEXT PRIMARY KEY REFERENCES knowledge_sources(id),
  name TEXT NOT NULL,
  media_type TEXT NOT NULL,
  bytes BLOB NOT NULL,
  byte_count INTEGER NOT NULL,
  saved_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS campaign_calculations (
  id TEXT PRIMARY KEY,
  campaign_id TEXT NOT NULL REFERENCES campaigns(id),
  kind TEXT NOT NULL,
  character_id TEXT,
  inputs_json TEXT NOT NULL,
  result_json TEXT NOT NULL,
  event_seq INTEGER NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS campaign_calculations_recent ON campaign_calculations(campaign_id,event_seq DESC);
