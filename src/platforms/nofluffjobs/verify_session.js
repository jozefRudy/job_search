// Proves the NoFluffJobs session is genuinely authenticated (not just a stale
// cookie) by hitting an authenticated endpoint with the HMAC-signed
// `authentication-candidate` header derived from the `nfj_token` cookie.
// Returns the HTTP status code, or 0 when the token is absent/malformed.
(async () => {
  const tokenEntry = document.cookie
    .split('; ')
    .find((r) => r.startsWith('nfj_token='));
  if (!tokenEntry) return 0;

  const token = decodeURIComponent(tokenEntry.split('=')[1]);
  const [session, secret] = token.split(':');
  if (!session || !secret) return 0;

  const path = '/candidates/my-applications';
  const key = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign']
  );
  const sigBuf = await crypto.subtle.sign(
    'HMAC',
    key,
    new TextEncoder().encode(encodeURI(path))
  );
  const sig = btoa(String.fromCharCode(...new Uint8Array(sigBuf)))
    .replace(/\+/g, '-')
    .replace(/\//g, '_');

  const res = await fetch(
    `/api${path}?page=0&limit=1&salaryCurrency=EUR&salaryPeriod=month&region=pl&language=en-GB`,
    { headers: { 'authentication-candidate': `${session}:${sig}` } }
  );
  return res.status;
})()
