(async () => {
  const page = __PAGE__;
  const limit = __LIMIT__;

  const sessionEntry = document.cookie
    .split('; ')
    .find((r) => r.startsWith('nfj_session='));
  const saltEntry = document.cookie
    .split('; ')
    .find((r) => r.startsWith('nfj_salt='));

  if (!sessionEntry || !saltEntry) {
    return { error: 'missing nofluffjobs auth cookies' };
  }

  const session = sessionEntry.split('=')[1];
  const salt = saltEntry.split('=')[1];

  const path = '/candidates/my-applications';
  const encodedPath = encodeURI(path);

  const key = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(salt),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign']
  );
  const sigBuf = await crypto.subtle.sign(
    'HMAC',
    key,
    new TextEncoder().encode(encodedPath)
  );
  const sig = btoa(String.fromCharCode(...new Uint8Array(sigBuf)))
    .replace(/\+/g, '-')
    .replace(/\//g, '_');

  const res = await fetch(
    `/api/candidates/my-applications?page=${page}&limit=${limit}&salaryCurrency=EUR&salaryPeriod=month&region=pl&language=en-GB`,
    { headers: { 'authentication-candidate': `${session}:${sig}` } }
  );

  if (!res.ok) {
    return {
      error: 'applications fetch failed',
      status: res.status,
      body: await res.text(),
    };
  }

  return await res.json();
})();
