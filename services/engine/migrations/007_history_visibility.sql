CREATE TABLE IF NOT EXISTS event_visibility (
  campaign_id TEXT NOT NULL REFERENCES campaigns(id),
  event_seq INTEGER NOT NULL REFERENCES events(seq),
  removed INTEGER NOT NULL CHECK(removed IN (0,1)),
  updated_at TEXT NOT NULL,
  PRIMARY KEY(campaign_id,event_seq)
);
