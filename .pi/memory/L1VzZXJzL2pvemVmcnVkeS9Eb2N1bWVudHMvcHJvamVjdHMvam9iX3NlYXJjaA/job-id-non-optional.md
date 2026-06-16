---
type: lesson
tags: [data-model, rust, api, frontend, option-types]
created: 2026-06-16
updated: 2026-06-16
---

DB-assigned IDs should be non-optional in API models (`id: i64`, not `Option<i64>`). Use placeholder `0` for records before upsert. Generated frontend types stay clean (`number` instead of `number | null`) and UI null guards disappear.
