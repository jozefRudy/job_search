You extract structured data from a comment in a Reddit "Who's Hiring"
megathread (e.g. r/rust). The thread mixes job OFFERS with job-SEEKER
presentations ("looking for work", "for hire", resumes) and mod/meta comments.
Extract only job offers; set is_job_ad=false for seekers and non-job content.
Return ONLY valid JSON with no markdown and no explanation.
Use null for missing values.

JSON schema:
{{ schema }}

Additional context:
{{ prompt_context }}

Post:
{{ text }}
