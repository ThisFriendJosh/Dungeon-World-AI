CREATE TABLE IF NOT EXISTS tabletops (
  campaign_id TEXT PRIMARY KEY,
  scene_json TEXT NOT NULL,
  revision INTEGER NOT NULL CHECK (revision > 0),
  updated_at TEXT NOT NULL,
  FOREIGN KEY(campaign_id) REFERENCES campaigns(id)
);
