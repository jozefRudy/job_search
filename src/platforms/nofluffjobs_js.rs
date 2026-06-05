//! JS snippets for NoFluffJobs browser scraping.

pub const SCRAPE_CARDS: &str = r#"
(() => {
    const text = (el, sel) => el.querySelector(sel)?.textContent?.trim() || null;

    return Array.from(document.querySelectorAll('a.posting-list-item')).map(el => {
        const title = (text(el, 'h3') || '').replace(/\s+NEW$/, '').trim();
        const href = el.href || '';
        const slug = href.split('/').pop() || '';

        return {
            external_id: slug,
            title,
            url: href,
            budget: text(el, '[data-cy="salary ranges on the job offer listing"]'),
            tags: Array.from(el.querySelectorAll('[data-cy="category name on the job offer listing"]'))
                .map(t => t.textContent.trim()).filter(Boolean)
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
document.querySelectorAll('a.posting-list-item').length
"#;

/// Extract total results count from the list header, e.g. "Jobs (135)" -> 135
pub const GET_TOTAL_RESULTS: &str = r#"
(() => {
    const match = document.querySelector('header.list-title')?.querySelector('span')?.textContent.match(/\((\d+)\)/);
    return match ? parseInt(match[1], 10) : null;
})()
"#;
