CREATE TABLE IF NOT EXISTS knowledge_sources (
  id TEXT PRIMARY KEY,
  campaign_id TEXT NOT NULL REFERENCES campaigns(id),
  title TEXT NOT NULL,
  source_ref TEXT NOT NULL,
  edition TEXT NOT NULL,
  gm_only INTEGER NOT NULL,
  chunk_count INTEGER NOT NULL,
  byte_count INTEGER NOT NULL,
  imported_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS knowledge_chunks (
  id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL REFERENCES knowledge_sources(id),
  ordinal INTEGER NOT NULL,
  body TEXT NOT NULL
);
CREATE VIRTUAL TABLE IF NOT EXISTS dm_reference_index USING fts5(
  campaign_id UNINDEXED, reference_id UNINDEXED, kind UNINDEXED,
  title, body, book UNINDEXED, edition UNINDEXED,
  gm_only UNINDEXED, source_ref UNINDEXED,
  tokenize='unicode61'
);
CREATE TABLE IF NOT EXISTS engine_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
INSERT INTO dm_reference_index(campaign_id,reference_id,kind,title,body,book,edition,gm_only,source_ref)
SELECT p.campaign_id,e.id,p.kind,e.name,e.content_json,e.book,e.source_edition,0,p.name
FROM compendium_entries e JOIN compendium_packs p ON p.id=e.pack_id
WHERE NOT EXISTS (SELECT 1 FROM engine_metadata WHERE key='dm-reference-v1');
INSERT OR IGNORE INTO engine_metadata(key,value) VALUES ('dm-reference-v1','1');
CREATE TRIGGER IF NOT EXISTS dm_compendium_insert AFTER INSERT ON compendium_entries BEGIN
  INSERT INTO dm_reference_index(campaign_id,reference_id,kind,title,body,book,edition,gm_only,source_ref)
  SELECT p.campaign_id,NEW.id,p.kind,NEW.name,NEW.content_json,NEW.book,NEW.source_edition,0,p.name
  FROM compendium_packs p WHERE p.id=NEW.pack_id;
END;
CREATE TRIGGER IF NOT EXISTS dm_compendium_delete AFTER DELETE ON compendium_entries BEGIN
  DELETE FROM dm_reference_index WHERE reference_id=OLD.id;
END;
CREATE TRIGGER IF NOT EXISTS dm_knowledge_insert AFTER INSERT ON knowledge_chunks BEGIN
  INSERT INTO dm_reference_index(campaign_id,reference_id,kind,title,body,book,edition,gm_only,source_ref)
  SELECT s.campaign_id,NEW.id,'campaign',s.title,NEW.body,s.title,s.edition,s.gm_only,s.source_ref
  FROM knowledge_sources s WHERE s.id=NEW.source_id;
END;
