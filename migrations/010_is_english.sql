-- English-language classification for job adverts.
ALTER TABLE jobs ADD COLUMN is_english BOOL NOT NULL DEFAULT 1;
