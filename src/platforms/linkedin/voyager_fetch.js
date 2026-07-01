(async function() {
  if (!document.cookie.includes('liap=true')) {
    throw new Error('LinkedIn login required. Open linkedin.com in Brave and log in.');
  }
  const url = '__VOYAGER_URL__';
  const csrf = (document.cookie.match(/JSESSIONID="([^"]+)"/) || [])[1] || '';
  const res = await fetch(url, {
    headers: {
      'csrf-token': csrf,
      'x-restli-protocol-version': '2.0.0',
      'accept': 'application/vnd.linkedin.normalized+json+2.1'
    },
    credentials: 'include'
  });
  if (!res.ok) {
    throw new Error('LinkedIn search failed: ' + res.status + ' ' + res.statusText);
  }
  const json = await res.json();
  const cards = json.data.elements.map(el => {
    const cardUrn = el.jobCardUnion['*jobPostingCard'];
    const card = json.included.find(i => i.entityUrn === cardUrn);
    if (!card) return null;
    const footer = card.footerItems?.find(f => f.type === 'LISTED_DATE');
    return {
      id: String(card.jobPostingUrn.replace('urn:li:fsd_jobPosting:', '')),
      title: card.title?.text || '',
      listedAt: footer?.timeAt || null
    };
  }).filter(Boolean);
  return { cards, total: json.data.paging?.total || 0 };
})()
