(async () => {
  const token = __TOKEN__;
  const jobseekerId = __JOBSEEKER_ID__;

  const res = await fetch(
    `https://job-activity.efinancialcareers.com/job-activities?jobseekerId=${jobseekerId}`,
    {
      headers: {
        Authorization: 'Bearer ' + token,
        Accept: 'application/json',
      },
    }
  );

  if (!res.ok) {
    return {
      error: 'applications fetch failed',
      status: res.status,
      body: await res.text(),
    };
  }

  const data = await res.json();
  const applied = (data.data || [])
    .filter((item) => item.status === 'APPLIED')
    .map((item) => {
      const job = item.job || {};
      const location = job.location || {};
      const match = (job.url || '').match(/\.id(\d+)(?:\?|$)/);
      return {
        internal_job_id: job.job_id || '',
        external_id: match ? match[1] : '',
        title: job.title || '',
        url: job.url || '',
        salary: job.salary_details || '',
        company: job.company_name || '',
        location: [location.city, location.country].filter(Boolean).join(', '),
        employment_type: [job.position_type, job.employment_type]
          .filter(Boolean)
          .join(' / '),
        applied_at_text: item.status_datetime || item.created_date || '',
      };
    });

  return { applied };
})();
