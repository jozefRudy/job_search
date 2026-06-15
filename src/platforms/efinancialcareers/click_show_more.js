(() => {
  const btn = [...document.querySelectorAll('button')].find(b => b.innerText.trim() === 'Show more');
  if (!btn) return false;
  const rect = btn.getBoundingClientRect();
  if (rect.height === 0 || rect.width === 0) return false;
  btn.scrollIntoView({ block: 'center', behavior: 'instant' });
  btn.click();
  return true;
})()
