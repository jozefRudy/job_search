// Verified facts:
// - in-page fetch with credentials:'include' returns 200 for .json endpoints
//   even anonymously (curl 403 is a UA/IP block, not auth) — but login check
//   still enforced per spec (rate limits saner when logged in)
// - search.json sort=new is BROKEN (ignores q). Use sort=top&t=all&limit=25,
//   filter titles /^Official \/r\/rust "Who's Hiring"/i, take max created_utc
// - comments/{id}.json?limit=500&depth=1 returns only top-level t1 children;
//   thread contains mod top-level comments (looking-for-work, meta) and
//   [deleted]/[removed] -> skip deleted, LLM is_job_ad rejects mod comments
(async function() {
  // login check via /api/me.json — document.cookie is useless here
  // (reddit_session/token_v2 are HttpOnly). Anonymous response has no data.name.
  const me = await (await fetch('/api/me.json', { credentials: 'include' })).json();
  if (!me?.data?.name) {
    throw new Error('Reddit login required. Open reddit.com in Brave and log in.');
  }
  const req = __REDDIT_REQUEST__;
  const res = await fetch(req.url, { credentials: 'include' });
  // do NOT throw on !res.ok — return { status, body } so Rust can backoff on 429
  return { status: res.status, body: res.ok ? await res.json() : null };
})()
