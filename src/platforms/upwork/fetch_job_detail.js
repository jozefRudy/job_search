(() => {
    const text = document.body.innerText;
    const rx = (pattern) => {
        const m = text.match(pattern);
        return m ? m[1]?.trim() : '';
    };

    const proposals = rx(/Proposals[:\s]*(?:Close[^\d]*)?(\d+\s+to\s+\d+|\d+)/i);
    const last_viewed_raw = rx(/Last viewed by client[:\s]*([^\n]+)/i);
    const last_viewed = last_viewed_raw ? last_viewed_raw.replace(/Close the tooltip.*$/, '').trim() : '';
    const interviewing = rx(/Interviewing[:\s]*(\d+)/i);
    const invites_sent = rx(/Invites sent[:\s]*(\d+)/i);
    const unanswered_invites = rx(/Unanswered invites[:\s]*(\d+)/i);
    const hires = rx(/Hires[:\s]*(\d+)/i);
    const project_type = rx(/Project type[:\s]*([^\n]+)/i);

    const liText = (selector) => {
        const el = document.querySelector(selector)?.closest('li');
        return el ? el.innerText.replace(/\s+/g, ' ').trim() : '';
    };

    const expText = liText('[data-cy="expertise"]');
    const experience_level = expText.match(/(Entry Level|Intermediate|Expert)/)?.[1] || '';

    const duration = liText('[data-cy^="duration"]')
        .replace(/\s*Duration\s*$/, '').trim();

    const hours_per_week = liText('[data-cy="clock-hourly"]')
        .replace(/\s*Hourly\s*$/, '').trim();

    let description = '';
    const descEl = document.querySelector('[data-test="Description"]')
        || document.querySelector('[data-test="job-description"]');
    if (descEl) {
        description = descEl.innerText?.trim() || '';
    }
    if (!description || description.length < 200) {
        const sections = Array.from(document.querySelectorAll('section'));
        for (const section of sections) {
            const t = section.innerText?.trim() || '';
            if (t.length > description.length
                && t.length > 200
                && !t.includes('Footer navigation')
                && !t.includes('Rating is')
                && !t.includes('To freelancer:')
                && !t.includes('Billed: $')) {
                description = t;
            }
        }
    }

    let exact_budget = '';
    const budgetLi = document.querySelector('[data-cy="clock-timelog"]')?.closest('li');
    if (budgetLi) {
        exact_budget = budgetLi.innerText.replace(/\s+/g, ' ').trim().replace(/\s*Hourly\s*$/, '').trim();
    }
    if (!exact_budget) {
        const budgetMatch = text.match(/\$\d+[\d,]*\.?\d*\s*[-]\s*\$\d+[\d,]*\.?\d*/)
            || text.match(/Budget[:\s]*([^\n]{0,50})/i);
        exact_budget = budgetMatch ? budgetMatch[0].replace(/\s+/g, ' ').trim() : '';
    }

    return { proposals, last_viewed, interviewing, invites_sent, unanswered_invites, description, exact_budget, experience_level, hires, project_type, duration, hours_per_week };
})()
