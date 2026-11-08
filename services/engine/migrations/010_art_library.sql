CREATE TABLE IF NOT EXISTS art_metadata (
 image_id TEXT PRIMARY KEY REFERENCES scene_images(id),
 campaign_id TEXT NOT NULL REFERENCES campaigns(id),
 metadata_json TEXT NOT NULL,
 source_svg TEXT,
 thumbnail BLOB,
 updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS token_artwork (
 image_id TEXT PRIMARY KEY REFERENCES generated_assets(image_id),
 campaign_id TEXT NOT NULL REFERENCES campaigns(id),
 overlay_png TEXT NOT NULL,
 recipe_json TEXT NOT NULL
);
