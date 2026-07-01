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
  const employmentType = employmentStatus?.localizedName || insightTexts[1] || '';

  const postedAt =
    jobPosting?.originalListedAt || jobPosting?.listedAt || jobPosting?.createdAt || 0;

  const location =
    geo?.defaultLocalizedName ||
    card?.tertiaryDescription?.text?.split('·')[0]?.trim() ||
    '';

  return {
    company: company?.name || card?.primaryDescription?.text || '',
    location,
    employment_type: employmentType,
    job_function: '',
    industries,
    description:
      jobPosting?.description?.text || jobDescription?.descriptionText?.text || '',
    salary: salary?.formattedBaseSalary || '',
    posted_at: postedAt,
  };
})(__JOB_CONFIG__)
