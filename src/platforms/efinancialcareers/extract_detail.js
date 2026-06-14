(() => {
  const desc = document.querySelector('efc-job-description');
  const text = document.body.innerText;
  const postedMatch = text.match(/Posted\s+([^\n]+?)\s*\n/);
  const postedLine = postedMatch ? postedMatch[0] : '';
  const salaryMatch = postedLine.match(/(?:per annum|per year|per month|per hour|per day|\b(?:USD|EUR|GBP|PLN|CHF|\$|€|£)\b).+/i);
  const salary = salaryMatch ? salaryMatch[0].trim() : '';
  const posted_at_text = postedMatch ? postedMatch[1].trim() : '';
  return {
    description: desc ? desc.innerText.trim() : '',
    salary,
    posted_at_text,
  };
})();
