//! JS snippets for NoFluffJobs browser scraping.

pub const SCRAPE_CARDS: &str = r#"
(() => {
    const cards = Array.from(document.querySelectorAll('a.posting-list-item'));
    return cards.map(el => {
        const titleEl = el.querySelector('h3');
        let title = titleEl?.textContent?.trim() || '';
        // Strip trailing NEW badge that Angular hydrates inside h3
        title = title.replace(/\s+NEW$/, '').trim();

        const href = el.href || '';
        const slug = href.split('/').pop() || '';

        // Salary from dedicated data-cy element
        const salaryEl = el.querySelector('[data-cy="salary ranges on the job offer listing"]');
        const budget = salaryEl?.textContent?.trim() || null;

        // Tags from dedicated category spans
        const tagEls = el.querySelectorAll('[data-cy="category name on the job offer listing"]');
        const tags = Array.from(tagEls).map(t => t.textContent.trim()).filter(Boolean);

        return {
            external_id: slug,
            title,
            url: href,
            budget,
            tags
        };
    });
})()
"#;

/// Finds "See more offers" button, scrolls into view, clicks it.
/// Returns true if button found and clicked, false otherwise.
pub const CLICK_LOAD_MORE: &str = r#"
(() => {
    const btn = document.querySelector('button[nfjloadmore]')
        || Array.from(document.querySelectorAll('button'))
            .find(el => /see more offers/i.test(el.textContent || ''));
    if (btn && !btn.disabled && btn.offsetParent !== null) {
        btn.scrollIntoView({ block: 'center' });
        btn.click();
        return true;
    }
    return false;
})()
"#;

pub const COUNT_CARDS: &str = r#"
(() => document.querySelectorAll('a.posting-list-item').length)()
"#;

/// Extract total results count from the list header, e.g. "Jobs (135)" -> 135
pub const GET_TOTAL_RESULTS: &str = r#"
(() => {
    const header = document.querySelector('header.list-title');
    if (!header) return null;
    const span = header.querySelector('span');
    if (!span) return null;
    const match = span.textContent.match(/\((\d+)\)/);
    return match ? parseInt(match[1], 10) : null;
})()
"#;
