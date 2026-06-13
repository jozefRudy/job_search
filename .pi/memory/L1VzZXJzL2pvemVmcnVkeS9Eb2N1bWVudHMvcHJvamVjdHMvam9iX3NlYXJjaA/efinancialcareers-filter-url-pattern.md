---
type: lesson
tags: [scraper, efinancialcareers, filters]
created: 2026-06-11
updated: 2026-06-13
---

eFinancialCareers filter URL pattern: multi-value filters use `|` separator (URL-encoded as `%7C`) for OR logic, and spaces in title values are encoded as literal `+`.

Example: `filters.fullNormalizedJobTitle=Developer%7CEngineer%7CQuant+Developer%7CPython%7CRust`
- Join CLI values with `,`: `"Developer,Engineer,Quant Developer,Python,Rust"`
- Use `urlencoding::encode`, then convert `%20` back to `+` to match the site's non-standard encoding.

Key filters:
- Work Arrangement: `filters.workArrangementType=REMOTE` (Remote/Hybrid/In-Office)
- Job Title (OR): `filters.fullNormalizedJobTitle=<pipe-separated titles>`
- Salary: `filters.minSalary=100000` (`maxSalary` is not required for remote searches)

`countryCode` is optional and not needed for remote searches; omitting it returns global remote results.

No job type (Permanent/Contract) filter available. Contract roles caught via keyword in search query if needed.
