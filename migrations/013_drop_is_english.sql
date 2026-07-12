-- Remove the is_english column and delete any previously-stored non-English jobs.
DELETE FROM jobs WHERE is_english = 0 OR is_english = false;

ALTER TABLE jobs DROP COLUMN is_english;
