// Fetch one SSR page and extract jobs from embedded Apollo state.
// config = { "url": pageUrl }
(async function (config) {
  const resp = await fetch(config.url, { credentials: "include" });
  if (!resp.ok) {
    return { error: `http ${resp.status}` };
  }
  // Out-of-range pages redirect to page 1 (no ?page in final URL) — signal end
  // instead of looping on duplicate page-1 jobs forever.
  const finalPage = new URL(resp.url).searchParams.get("page");
  if (String(config.page) !== (finalPage ?? "1")) {
    return { jobs: [], end: true };
  }
  const html = await resp.text();

  const marker = '"apolloState":{"data":{';
  const i = html.indexOf(marker);
  if (i < 0) {
    return { error: "no apolloState — not logged in or markup changed" };
  }

  // Brace-match the data object (avoids regex on JSON). String-aware: braces
  // inside quoted values (code snippets, `${}` in descriptions) must not
  // affect depth.
  const start = i + marker.length - 1;
  let depth = 0;
  let end = start;
  let inString = false;
  let escaped = false;
  for (; end < html.length; end++) {
    const c = html[end];
    if (inString) {
      if (escaped) escaped = false;
      else if (c === "\\") escaped = true;
      else if (c === '"') inString = false;
      continue;
    }
    if (c === '"') inString = true;
    else if (c === "{") depth++;
    else if (c === "}") {
      depth--;
      if (depth === 0) {
        end++;
        break;
      }
    }
  }
  let data;
  try {
    data = JSON.parse(html.slice(start, end));
  } catch (e) {
    return { error: `apolloState JSON parse failed: ${e.message}` };
  }

  // jobId -> startup reverse map (jobs have no startup field; one startup can
  // highlight 2+ jobs; startup may appear after the job in key order).
  const startupByJobId = {};
  for (const [key, value] of Object.entries(data)) {
    if (!key.startsWith("StartupResult:")) continue;
    for (const ref of value.highlightedJobListings || []) {
      const jobId = ref.__ref?.split(":")[1];
      if (jobId) startupByJobId[jobId] = value;
    }
  }

  const jobs = [];
  for (const [key, j] of Object.entries(data)) {
    if (!key.startsWith("JobListingSearchResult:")) continue;
    const s = startupByJobId[j.id];
    jobs.push({
      id: j.id,
      title: j.title,
      slug: j.slug,
      description: j.description,
      compensation: j.compensation ?? null,
      job_type: j.jobType,
      live_start_at: j.liveStartAt ?? null,
      location_names: j.locationNames ?? [],
      remote: j.remote,
      remote_kind: j.remoteConfig?.kind ?? null,
      accepted_remote_location_names: j.acceptedRemoteLocationNames ?? [],
      years_experience_min: j.yearsExperienceMin ?? null,
      years_experience_max: j.yearsExperienceMax ?? null,
      primary_role_title: j.primaryRoleTitle ?? null,
      company_name: s?.name ?? "",
      company_slug: s?.slug ?? null,
      company_size: s?.companySize ?? null,
      company_high_concept: s?.highConcept ?? null,
      company_logo_url: s?.logoUrl ?? null,
    });
  }
  return { jobs };
})(__CONFIG__)
