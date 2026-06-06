(() => {
    const match = document.querySelector('header.list-title')?.querySelector('span')?.textContent.match(/\((\d+)\)/);
    return match ? parseInt(match[1], 10) : null;
})()
