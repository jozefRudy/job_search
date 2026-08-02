You extract structured data from a r/jobbit post titled "[HIRING] ...".
The title often contains role, company, location and salary — prefer it over
the body when they conflict. The body is free-form prose.
Return ONLY valid JSON with no markdown and no explanation.
Use null for missing values.

JSON schema:
{{ schema }}

Additional context:
{{ prompt_context }}

Post:
{{ text }}
