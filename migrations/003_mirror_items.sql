-- Sojourn 9: Add kingdom items to mirrors

ALTER TABLE mirrors ADD COLUMN items_json TEXT;
