(() => {
  const lists = window.__NUXT__?.state?.lists;
  if (!lists) throw new Error('__NUXT__ state not available');

  const submitted = lists.submittedList;
  if (!submitted) throw new Error('submittedList not found');

  const items = submitted.items || [];
  return {
    page: submitted.paging?.page ?? 0,
    total: submitted.paging?.total ?? 0,
    itemsPerPage: submitted.paging?.itemsPerPage ?? 10,
    items: items.map(i => ({
      openingUID: i.openingUID,
      applicationUID: i.applicationUID,
      title: i.title,
      createdTs: i.auditDetails?.createdTs || null,
    })),
  };
})()
