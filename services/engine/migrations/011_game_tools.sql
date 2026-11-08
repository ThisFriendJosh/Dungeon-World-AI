CREATE TABLE IF NOT EXISTS game_tools (
 campaign_id TEXT PRIMARY KEY REFERENCES campaigns(id),
 state_json TEXT NOT NULL,
 revision INTEGER NOT NULL,
 updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS scrying_bookmarks (
 id TEXT PRIMARY KEY,
 campaign_id TEXT NOT NULL REFERENCES campaigns(id),
 title TEXT NOT NULL,
 url TEXT NOT NULL,
 created_at TEXT NOT NULL,
 UNIQUE(campaign_id,url)
);
