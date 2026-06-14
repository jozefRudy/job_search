---
type: lesson
tags: [nofluffjobs, auth, browser, cookies]
created: 2026-06-14
updated: 2026-06-14
---

NoFluffJobs sync auth changed: `nfj_salt` cookie replaced by `nfj_token=<session>:<secret>`. Update `fetch_applications.js` to use the secret part of `nfj_token` as HMAC key instead of `nfj_salt`. Avoid calling `set_currency_cookie` in `sync_applications` because CDP cookie manipulation can clobber existing session/auth cookies.
