//! JS snippets for NoFluffJobs browser scraping.

pub const SCRAPE_CARDS: &str = r#"
(() => {
    const cards = Array.from(document.querySelectorAll('a.posting-list-item'));
    return cards.map(el => {
        const titleEl = el.querySelector('h3');
        const title = titleEl?.textContent?.trim() || '';
        const href = el.href || '';
        const slug = href.split('/').pop() || '';

        const allText = el.innerText.split('\n').map(t => t.trim()).filter(Boolean);

        // Find salary line
        let budget = null;
        for (const text of allText) {
            if (/\d+[\s\u00a0]*\d*\s*[–-]\s*\d+[\s\u00a0]*\d*\s*(PLN|EUR|USD)/.test(text)) {
                budget = text;
                break;
            }
        }

        // Tags: lines between title and salary, excluding "Save this job offer"
        const tags = [];
        let started = false;
        for (const text of allText) {
            if (text === title) { started = true; continue; }
            if (text === budget) break;
            if (started && text && text !== 'Save this job offer' && !tags.includes(text)) {
                tags.push(text);
            }
        }

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
