CREATE TABLE IF NOT EXISTS generated_assets (
  image_id TEXT PRIMARY KEY REFERENCES scene_images(id),
  campaign_id TEXT NOT NULL REFERENCES campaigns(id),
  spec_json TEXT NOT NULL,
  svg_text TEXT NOT NULL,
  created_at TEXT NOT NULL
);
