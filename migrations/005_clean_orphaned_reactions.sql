DELETE FROM reactions WHERE job_id NOT IN (SELECT id FROM jobs);
