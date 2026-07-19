(async function (config) {
  if (!document.cookie.includes('liap=true')) {
    throw new Error('LinkedIn login required. Open linkedin.com in Brave and log in.');
  }
  const csrf = (document.cookie.match(/JSESSIONID="([^"]+)"/) || [])[1] || '';
  const { baseUrl, detailQueryId, jobQueryId, jobPostingUrn, cardSectionTypes } = config;

  const cards = cardSectionTypes.join(',');
  const detailVariables = `(cardSectionTypes:List(${cards}),jobPostingUrn:${encodeURIComponent(jobPostingUrn)},includeSecondaryActionsV2:true)`;
  const detailUrl = `${baseUrl}?includeWebMetadata=true&variables=${detailVariables}&queryId=${detailQueryId}`;
  const jobVariables = `(jobPostingUrn:${encodeURIComponent(jobPostingUrn)})`;
  const jobUrl = `${baseUrl}?includeWebMetadata=true&variables=${jobVariables}&queryId=${jobQueryId}`;

  const headers = {
    'csrf-token': csrf,
    'x-restli-protocol-version': '2.0.0',
    'accept': 'application/vnd.linkedin.normalized+json+2.1',
  };

  const [detailRes, jobRes] = await Promise.all([
    fetch(detailUrl, { headers, credentials: 'include' }),
    fetch(jobUrl, { headers, credentials: 'include' }),
  ]);

  if (!detailRes.ok) {
    throw new Error('LinkedIn detail sections failed: ' + detailRes.status + ' ' + detailRes.statusText);
  }
  if (!jobRes.ok) {
    throw new Error('LinkedIn job posting failed: ' + jobRes.status + ' ' + jobRes.statusText);
  }

  const detailJson = await detailRes.json();
  const jobJson = await jobRes.json();

  const detailInc = detailJson.included || [];
  const jobInc = jobJson.included || [];
  const detailByType = (t) => detailInc.find((i) => i.$type === t);
  const jobByType = (t) => jobInc.find((i) => i.$type === t);
  const jobByUrn = (u) => jobInc.find((i) => i.entityUrn === u);
  const detailByUrn = (u) => detailInc.find((i) => i.entityUrn === u);

  const jobPosting = jobByType('com.linkedin.voyager.dash.jobs.JobPosting');
  const jobDescription = detailByType('com.linkedin.voyager.dash.jobs.JobDescription');
  const card = detailByType('com.linkedin.voyager.dash.jobs.JobPostingCard');
  const salary = detailByType('com.linkedin.voyager.dash.salary.SalaryInsights');
  const company =
    detailByType('com.linkedin.voyager.dash.organization.Company') ||
    jobByType('com.linkedin.voyager.dash.organization.Company');
  const geo =
    detailByUrn(jobPosting?.['*location']) || jobByUrn(jobPosting?.['*location']);

  const employmentStatus = jobByUrn(jobPosting?.['*employmentStatus']);
  const industryUrns = jobPosting?.['*industryV2Taxonomy'] || [];
  const industries = industryUrns
    .map((u) => jobByUrn(u)?.name || '')
    .filter(Boolean)
    .join(', ');

  const insights = card?.jobInsightsV2ResolutionResults || [];
  const insightTexts = insights
    .map((i) => i.jobInsightViewModel?.description?.[0]?.text?.text)
    .filter(Boolean);
  const knownWorkplaceTypes = ['Remote', 'Hybrid', 'On-site', 'Onsite'];
  const workplaceType =
    insightTexts.find((t) =>
      knownWorkplaceTypes.some(
        (w) => t.toLowerCase() === w.toLowerCase() || t.toLowerCase().includes(w.toLowerCase())
      )
    ) ||
    card?.navigationBarSubtitle?.match(/\(([^)]+)\)/)?.[1] ||
    '';
  const employmentType = employmentStatus?.localizedName || insightTexts[1] || '';

  const postedAt =
    jobPosting?.originalListedAt || jobPosting?.listedAt || jobPosting?.createdAt || 0;

  const location =
    geo?.defaultLocalizedName ||
    card?.tertiaryDescription?.text?.split('·')[0]?.trim() ||
    '';

  function formatTextViewModelAsMarkdown(text, attributesV2) {
    if (!text || !attributesV2 || attributesV2.length === 0) return text;

    const chars = Array.from(text);

    const attrs = attributesV2
      .map((a) => ({ start: a.start, end: a.start + a.length, style: a.detailData?.style }))
      .filter((a) => a.start >= 0 && a.end <= chars.length);

    const paragraphs = attrs
      .filter((a) => a.style === 'PARAGRAPH' && a.end - a.start > 1)
      .sort((a, b) => a.start - b.start);

    const listItems = attrs
      .filter((a) => a.style === 'LIST_ITEM')
      .sort((a, b) => a.start - b.start);

    const inlineStyles = attrs
      .filter((a) => a.style === 'BOLD' || a.style === 'ITALIC')
      .map((a) => ({ start: a.start, end: a.end, style: a.style.toLowerCase() }))
      .sort((a, b) => a.start - b.start);

    const contents = [...paragraphs, ...listItems].sort((a, b) => a.start - b.start);

    const blocks = [];
    let currentList = [];
    let prevEnd = 0;

    for (const item of contents) {
      if (item.start < prevEnd) continue;
      const formatted = formatInline(chars, item.start, item.end, inlineStyles);

      if (item.style === 'LIST_ITEM') {
        currentList.push('- ' + formatted.replace(/^•\s*/, ''));
      } else {
        if (currentList.length > 0) {
          blocks.push(currentList.join('\n'));
          currentList = [];
        }
        blocks.push(formatted);
      }
      prevEnd = item.end;
    }

    if (currentList.length > 0) {
      blocks.push(currentList.join('\n'));
    }

    return blocks.join('\n\n');
  }

  function formatInline(chars, segmentStart, segmentEnd, inlineStyles) {
    const segmentLength = segmentEnd - segmentStart;
    const relevant = inlineStyles
      .filter((a) => a.end > segmentStart && a.start < segmentEnd)
      .map((a) => ({
        start: Math.max(0, a.start - segmentStart),
        end: Math.min(segmentLength, a.end - segmentStart),
        style: a.style,
      }))
      .filter((a) => a.start < a.end)
      .sort((a, b) => a.start - b.start);

    if (relevant.length === 0) {
      return chars.slice(segmentStart, segmentEnd).join('');
    }

    let result = '';
    let lastPos = 0;
    for (const a of relevant) {
      if (a.start < lastPos) continue;
      const marker = a.style === 'bold' ? '**' : '*';
      result += chars.slice(segmentStart + lastPos, segmentStart + a.start).join('');
      result += marker + chars.slice(segmentStart + a.start, segmentStart + a.end).join('') + marker;
      lastPos = a.end;
    }
    result += chars.slice(segmentStart + lastPos, segmentEnd).join('');
    return result;
  }

  const descriptionTv = jobPosting?.description || jobDescription?.descriptionText || {};
  const description = formatTextViewModelAsMarkdown(descriptionTv.text, descriptionTv.attributesV2);

  return {
    company: company?.name || card?.primaryDescription?.text || '',
    location,
    workplace_type: workplaceType,
    employment_type: employmentType,
    job_function: '',
    industries,
    description,
    salary: salary?.formattedBaseSalary || '',
    posted_at: postedAt,
  };
})(__JOB_CONFIG__)
