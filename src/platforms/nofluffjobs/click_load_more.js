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
