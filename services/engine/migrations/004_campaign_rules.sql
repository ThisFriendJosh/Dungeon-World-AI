CREATE TABLE IF NOT EXISTS campaign_rules (
  campaign_id TEXT PRIMARY KEY,
  config_json TEXT NOT NULL,
  revision INTEGER NOT NULL CHECK(revision > 0),
  updated_at TEXT NOT NULL,
  FOREIGN KEY(campaign_id) REFERENCES campaigns(id)
);
INSERT OR IGNORE INTO campaign_rules(campaign_id,config_json,revision,updated_at)
SELECT id, '{"schemaVersion":1,"profile":"5e-manual","houseRules":{"flankingAdvantage":false,"potionAsBonusAction":true,"critMaxOneDie":false}}', 1, updated_at FROM campaigns;
