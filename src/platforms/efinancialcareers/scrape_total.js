(() => {
  const text = document.body.innerText;
  if (/No jobs found/i.test(text)) {
    return 0;
  }
  const m = text.match(/^[^\n]*?\bjobs?\b[^\n]*?\((\d{1,6})\)/mi);
  return m ? parseInt(m[1], 10) : null;
})()
