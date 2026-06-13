(() => {
  return [...document.querySelectorAll('efc-job-search-results efc-job-card')].map(card => {
    const a = card.querySelector('a.job-title');
    const text = card.innerText;
    const salaryMatch = text.match(/(?:USD|EUR|GBP|PLN|CHF|\$|€|£)[^\n]+(?:per annum|per year|\/hr|hour|\b)/i)
      || text.match(/\b(Competitive|High salary|Negotiable|DOE|N\/A)\b/i);
    const postedMatch = text.match(/Posted\s+([^\n]+)/i);
    return {
      external_id: a?.id || '',
      title: a?.querySelector('h3')?.textContent?.trim() || '',
      url: a?.href || '',
      salary: salaryMatch ? salaryMatch[0].trim() : '',
      posted_at_text: postedMatch ? postedMatch[1].trim() : ''
    };
  });
})()
