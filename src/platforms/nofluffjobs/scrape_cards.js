(() => {
    const text = (el, sel) => el.querySelector(sel)?.textContent?.trim() || null;

    return Array.from(document.querySelectorAll('a.posting-list-item')).map(el => {
        const title = (text(el, 'h3') || '').replace(/\s+NEW$/, '').trim();
        const href = el.href || '';
        const slug = href.split('/').pop() || '';

        return {
            external_id: slug.toLowerCase(),
            title,
            url: href,
            budget: text(el, '[data-cy="salary ranges on the job offer listing"]'),
            tags: Array.from(el.querySelectorAll('[data-cy="category name on the job offer listing"]'))
                .map(t => t.textContent.trim()).filter(Boolean)
        };
    });
})()
