(async function (config) {
  const entry = performance
    .getEntriesByType("resource")
    .find((e) => e.name.includes("algolia.net/1/indexes") && e.name.includes("x-algolia-api-key"));
  if (!entry) {
    return { error: "no algolia key — log in at workatastartup.com and load /companies" };
  }
  const params = `query=&hitsPerPage=100&page=${config.page}&filters=${encodeURIComponent(config.filters)}`;
  const resp = await fetch(entry.name, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: JSON.stringify({ requests: [{ indexName: config.indexName, params }] }),
  });
  if (!resp.ok) {
    return { error: `algolia http ${resp.status}` };
  }
  const data = await resp.json();
  const result = data.results?.[0];
  if (!result) {
    return { error: "empty algolia response" };
  }
  return { hits: result.hits, nbPages: result.nbPages };
})(__CONFIG__)
