-- Remove reactions whose job was already deleted.
-- Keeps the reactions table consistent regardless of past cascade settings.
DELETE FROM reactions WHERE job_id NOT IN (SELECT id FROM jobs);
