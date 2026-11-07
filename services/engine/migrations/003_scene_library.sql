CREATE TABLE IF NOT EXISTS tabletop_scenes (
  campaign_id TEXT NOT NULL,
  scene_id TEXT NOT NULL,
  scene_json TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (campaign_id, scene_id),
  FOREIGN KEY (campaign_id) REFERENCES campaigns(id)
);
INSERT OR IGNORE INTO tabletop_scenes (campaign_id, scene_id, scene_json, updated_at)
SELECT campaign_id, json_extract(scene_json, '$.id'), scene_json, updated_at FROM tabletops;
CREATE TABLE IF NOT EXISTS scene_images (
  id TEXT PRIMARY KEY,
  campaign_id TEXT NOT NULL,
  name TEXT NOT NULL,
  width INTEGER NOT NULL,
  height INTEGER NOT NULL,
  data BLOB NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (campaign_id) REFERENCES campaigns(id)
);
