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
    const posted_at_text = rx(/Posted[:\s]*([^\n]+)/i);

    const itemText = (selector, label, require) => {
        const el = document.querySelector(selector);
        if (!el) return '';
        const parent = el.parentElement;
        if (!parent) return '';
        if (require && !parent.innerText.includes(require)) return '';
        let text = parent.innerText.replace(/\s+/g, ' ').trim();
        if (label) {
            text = text.replace(new RegExp(`\\s*${label}\\s*$`), '').trim();
        }
        return text;
    };

    const experience_level = itemText('[data-cy="expertise"]', '')
        .match(/(Entry Level|Intermediate|Expert)/)?.[1] || '';

    const duration = itemText('[data-cy^="duration"]', 'Duration');

    const hours_per_week = itemText('[data-cy="clock-hourly"]', 'Hourly');

    let description = '';
    const descEl = document.querySelector('[data-test="Description"]')
        || document.querySelector('[data-test="job-description"]');
    if (descEl) {
        description = descEl.innerText?.trim() || '';
    }
    if (!description) {
        const sections = Array.from(document.querySelectorAll('section'));
        for (const section of sections) {
            const t = section.innerText?.trim() || '';
            if (t.length > description.length
                && t.length > 200
                && !t.includes('Footer navigation')
                && !t.includes('Rating is')
                && !t.includes('To freelancer:')
                && !t.includes('Billed: $')
                && !t.includes('Apply now')
                && !t.includes('Save job')
                && !t.includes('Send a proposal')
                && !t.includes('About the client')) {
                description = t;
            }
        }
    }

    let exact_budget = itemText('[data-cy="clock-timelog"]', 'Hourly', 'Hourly');
    if (!exact_budget) {
        const html = document.documentElement.innerHTML;
        const budgetHidden = html.match(/hourlyBudgetType[^,]*,\s*"([^"]+)"/);
        if (budgetHidden && budgetHidden[1] === 'NOT_PROVIDED') {
            exact_budget = 'Budget hidden';
        }
    }

    const tags = Array.from(document.querySelectorAll('a[href*="ontology_skill_uid"]'))
        .map(a => a.innerText?.trim())
        .filter(Boolean);

    return { proposals, last_viewed, interviewing, invites_sent, unanswered_invites, description, exact_budget, experience_level, hires, project_type, duration, hours_per_week, tags, posted_at_text };
})()
