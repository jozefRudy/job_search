(() => {
  const pd = window.__NUXT__?.state?.['proposal-details'];
  if (!pd) return '';
  return pd.proposalDetailsV3Response?.application?.coverLetter || '';
})()
