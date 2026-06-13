---
type: lesson
tags: [scraper, efinancialcareers, filters]
created: 2026-06-11
updated: 2026-06-13
---

eFinancialCareers filter URL pattern: multi-value filters use `|` separator (URL-encoded as `%7C`) for OR logic.

Example: `filters.fullNormalizedJobTitle=Developer%7CEngineer%7CQuant+Developer%7CPython%7CRust`
- Join CLI values with `,`: `"Developer,Engineer,Quant Developer,Python,Rust"`
- Transform: replace space with `+` in multi-word titles, join with `|`, then `%7C`-encode pipes.

Key filters:
- Work Arrangement: `filters.workArrangementType=REMOTE` (Remote/Hybrid/In-Office)
- Job Title (OR): `filters.fullNormalizedJobTitle=<pipe-separated titles>`
- Country: `countryCode=US` (separate param, not inside filters)
- Salary range: `filters.minSalary=100000&filters.maxSalary=900000` (max required by site when min provided)

No job type (Permanent/Contract) filter available. Contract roles caught via keyword in search query if needed.
