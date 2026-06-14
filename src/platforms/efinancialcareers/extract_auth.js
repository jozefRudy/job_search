(() => {
  const tokenEntry = document.cookie
    .split(';')
    .find((c) => c.trim().startsWith('myEfcCookieAuth='));

  if (!tokenEntry) {
    return { error: 'missing efinancialcareers auth cookie' };
  }

  const token = tokenEntry.slice('myEfcCookieAuth='.length).trim();
  const payload = JSON.parse(atob(token.split('.')[1]));

  return {
    token,
    jobseeker_id: payload.jobseekerId || '',
  };
})();
