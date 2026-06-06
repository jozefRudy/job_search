(() => {
    return Array.from(document.querySelectorAll("article[data-test='JobTile']")).map(el => {
        const titleLink = el.querySelector('a');
        const budgetEl = el.querySelector("[data-test='job-type-label']");
        const timeEl = el.querySelector('small[data-test="job-pubilshed-date"]');
        const skillsEls = el.querySelectorAll("[data-test='token']");
        const uid = el.getAttribute("data-ev-job-uid");
        return {
            external_id: uid || "",
            title: titleLink?.textContent?.trim() || "",
            description: null,
            url: titleLink?.href ? new URL(titleLink.href, location.href).href : "",
            budget: budgetEl?.textContent?.trim() || null,
            posted_at_text: timeEl?.textContent?.trim() || null,
            tags: Array.from(skillsEls).map(s => s.textContent.trim()).filter(Boolean)
        };
    });
})()
