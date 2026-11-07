CREATE TABLE IF NOT EXISTS compendium_packs (
 id TEXT PRIMARY KEY,
 campaign_id TEXT NOT NULL,
 name TEXT NOT NULL,
 kind TEXT NOT NULL,
 entry_count INTEGER NOT NULL,
 byte_count INTEGER NOT NULL,
 imported_at TEXT NOT NULL,
 FOREIGN KEY(campaign_id) REFERENCES campaigns(id)
);
CREATE TABLE IF NOT EXISTS compendium_entries (
 id TEXT PRIMARY KEY,
 pack_id TEXT NOT NULL,
 name TEXT NOT NULL,
 book TEXT NOT NULL,
 publisher TEXT NOT NULL,
 source_edition TEXT NOT NULL,
 content_json TEXT NOT NULL,
 FOREIGN KEY(pack_id) REFERENCES compendium_packs(id)
);
CREATE INDEX IF NOT EXISTS compendium_by_pack ON compendium_entries(pack_id);
CREATE INDEX IF NOT EXISTS compendium_by_name ON compendium_entries(name COLLATE NOCASE);
