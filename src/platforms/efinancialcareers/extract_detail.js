(() => {
  const desc = document.querySelector('efc-job-description');
  const text = document.body.innerText;
  const metaMatch = text.match(/Posted[^\n]*Remote Job[^\n]*\n/);
  const salaryMatch = metaMatch && metaMatch[0].match(/(?:Permanent|Contract|Full time|Part time)\s+(.+?)(?:\n|$)/);
  const salary = salaryMatch ? salaryMatch[1].trim() : '';
  const postedMatch = text.match(/Posted\s+([^\n]+?)\s*\n/);
  const posted_at_text = postedMatch ? postedMatch[1].trim() : '';
  return {
    description: desc ? desc.innerText.trim() : '',
    salary,
    posted_at_text,
  };
})();
