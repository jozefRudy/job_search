---
type: context
created: 2026-06-11
updated: 2026-06-11
---

`Db::set_applied` should take explicit non-optional `applied_at: NaiveDateTime` parameter. When syncing from Upwork, exact timestamp from `auditDetails.createdTs` is always known. For manual `react apply`, pass `Utc::now().naive_utc()`. No `DEFAULT CURRENT_TIMESTAMP` or `COALESCE` needed — always use provided timestamp.
