// TODO(phase3): args: (filters: string, page: number, indexName: string)
// 1. Find secured-key Algolia URL in performance.getEntriesByType('resource')
//    (name contains "algolia.net/1/indexes" and "x-algolia-api-key").
//    If none, return { error: "no algolia key — log in at workatastartup.com" }.
// 2. POST to that URL with body
//    {requests:[{indexName, params: `query=&hitsPerPage=100&page=${page}&filters=${encodeURIComponent(filters)}`}]}
// 3. Return {hits, nbPages} from response.results[0]; on HTTP error return {error}.
