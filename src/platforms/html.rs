use html2text::from_read;

const HTML_TO_MD_WIDTH: usize = 120;

#[must_use]
pub fn html_to_md(html: &str) -> Option<String> {
    if html.trim().is_empty() {
        return None;
    }
    from_read(html.as_bytes(), HTML_TO_MD_WIDTH)
        .map(|t| t.trim().to_string())
        .ok()
        .filter(|t| !t.is_empty())
}
