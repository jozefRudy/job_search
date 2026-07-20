use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TextViewModel {
    pub text: String,
    #[serde(rename = "attributesV2")]
    pub attributes: Vec<TextAttribute>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TextAttribute {
    pub start: usize,
    pub length: usize,
    #[serde(rename = "detailData")]
    pub detail_data: TextAttributeDetail,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TextAttributeDetail {
    pub style: Option<String>,
    pub hyperlink: Option<String>,
    #[serde(rename = "textLink")]
    pub text_link: Option<TextLink>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TextLink {
    pub url: Option<String>,
    #[serde(rename = "redirectUrl")]
    pub redirect_url: Option<String>,
}

impl TextAttributeDetail {
    fn style_str(&self) -> Option<&str> {
        self.style.as_deref()
    }

    fn url(&self) -> Option<&str> {
        self.hyperlink.as_deref().or(self
            .text_link
            .as_ref()
            .and_then(|l| l.url.as_deref().or(l.redirect_url.as_deref())))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DescriptionSection {
    pub header: Option<TextViewModel>,
    pub body: TextViewModel,
}

#[derive(Debug, Clone)]
struct Block {
    start: usize,
    end: usize,
    kind: BlockKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Paragraph,
    ListItem,
}

#[derive(Debug, Clone, Copy)]
struct InlineStyle<'a> {
    start: usize,
    end: usize,
    kind: InlineStyleKind<'a>,
}

#[derive(Debug, Clone, Copy)]
enum InlineStyleKind<'a> {
    Bold,
    Italic,
    Link(&'a str),
}

pub fn format_text_view_model(tv: &TextViewModel) -> String {
    format_text(&tv.text, &tv.attributes)
}

fn format_text(text: &str, attributes: &[TextAttribute]) -> String {
    if text.is_empty() {
        return String::new();
    }

    let chars: Vec<char> = text.chars().collect();

    let mut valid_attrs: Vec<_> = attributes
        .iter()
        .filter(|a| a.start < a.start + a.length && a.start + a.length <= chars.len())
        .collect();
    valid_attrs.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then_with(|| a.length.cmp(&b.length).reverse())
    });

    let blocks: Vec<Block> = build_blocks(&chars, &valid_attrs);

    let inline_styles: Vec<InlineStyle> = valid_attrs
        .iter()
        .filter_map(|a| {
            let style = a.detail_data.style_str()?;
            let kind = match style {
                "BOLD" => InlineStyleKind::Bold,
                "ITALIC" => InlineStyleKind::Italic,
                "URL" | "HYPERLINK" => InlineStyleKind::Link(a.detail_data.url()?),
                _ => return None,
            };
            Some(InlineStyle {
                start: a.start,
                end: a.start + a.length,
                kind,
            })
        })
        .collect();

    let mut rendered: Vec<String> = Vec::new();
    let mut current_list: Vec<String> = Vec::new();

    for block in blocks {
        let formatted = format_inline(&chars, block.start, block.end, &inline_styles)
            .trim()
            .to_string();
        if formatted.is_empty() {
            continue;
        }

        match block.kind {
            BlockKind::ListItem => {
                current_list.push(format!(
                    "- {}",
                    formatted
                        .strip_prefix('•')
                        .unwrap_or(&formatted)
                        .trim_start()
                ));
            }
            BlockKind::Paragraph => {
                if !current_list.is_empty() {
                    rendered.push(current_list.join("\n"));
                    current_list.clear();
                }
                rendered.push(formatted);
            }
        }
    }

    if !current_list.is_empty() {
        rendered.push(current_list.join("\n"));
    }

    rendered.join("\n\n")
}

fn build_blocks(chars: &[char], attributes: &[&TextAttribute]) -> Vec<Block> {
    let block_attrs: Vec<_> = attributes
        .iter()
        .filter(|a| {
            let style = a.detail_data.style_str();
            style == Some("PARAGRAPH") || style == Some("LIST_ITEM")
        })
        .map(|a| Block {
            start: a.start,
            end: a.start + a.length,
            kind: if a.detail_data.style_str() == Some("LIST_ITEM") {
                BlockKind::ListItem
            } else {
                BlockKind::Paragraph
            },
        })
        .collect();

    let mut blocks: Vec<Block> = Vec::new();
    let mut pos = 0;

    for attr in &block_attrs {
        if attr.start > pos {
            blocks.push(Block {
                start: pos,
                end: attr.start,
                kind: BlockKind::Paragraph,
            });
        }
        blocks.push(Block {
            start: attr.start,
            end: attr.end,
            kind: attr.kind,
        });
        pos = pos.max(attr.end);
    }

    if pos < chars.len() {
        blocks.push(Block {
            start: pos,
            end: chars.len(),
            kind: BlockKind::Paragraph,
        });
    }

    blocks
}

fn format_inline(
    chars: &[char],
    segment_start: usize,
    segment_end: usize,
    inline_styles: &[InlineStyle],
) -> String {
    let segment_styles: Vec<_> = inline_styles
        .iter()
        .filter(|s| s.end > segment_start && s.start < segment_end)
        .map(|s| InlineStyle {
            start: s.start.max(segment_start),
            end: s.end.min(segment_end),
            kind: s.kind,
        })
        .filter(|s| s.start < s.end)
        .collect();

    if segment_styles.is_empty() {
        return chars[segment_start..segment_end].iter().collect();
    }

    let mut boundaries: Vec<usize> = Vec::new();
    boundaries.push(segment_start);
    for s in &segment_styles {
        boundaries.push(s.start);
        boundaries.push(s.end);
    }
    boundaries.push(segment_end);
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut result = String::new();
    for window in boundaries.windows(2) {
        let start = window[0];
        let end = window[1];
        if start < segment_start || end > segment_end || start >= end {
            continue;
        }

        let text: String = chars[start..end].iter().collect();
        let trimmed = text.trim_end();
        if trimmed.is_empty() {
            result.push_str(&text);
            continue;
        }
        let trailing = &text[trimmed.len()..];

        let active: Vec<_> = segment_styles
            .iter()
            .filter(|s| s.start <= start && s.end >= end)
            .map(|s| s.kind)
            .collect();

        let is_link = active.iter().any(|k| matches!(k, InlineStyleKind::Link(_)));
        let is_bold = active.iter().any(|k| matches!(k, InlineStyleKind::Bold));
        let is_italic = active.iter().any(|k| matches!(k, InlineStyleKind::Italic));

        let styled = if is_link {
            let url = active
                .iter()
                .find_map(|k| match k {
                    InlineStyleKind::Link(u) => Some(*u),
                    _ => None,
                })
                .unwrap_or("");
            if is_bold && is_italic {
                format!("[***{trimmed}***]({url})")
            } else if is_bold {
                format!("[**{trimmed}**]({url})")
            } else if is_italic {
                format!("[*{trimmed}*]({url})")
            } else {
                format!("[{trimmed}]({url})")
            }
        } else if is_bold && is_italic {
            format!("***{trimmed}***")
        } else if is_bold {
            format!("**{trimmed}**")
        } else if is_italic {
            format!("*{trimmed}*")
        } else {
            trimmed.to_string()
        };

        result.push_str(&styled);
        result.push_str(trailing);
    }

    result
}

pub fn description_sections_to_markdown(sections: &[DescriptionSection]) -> String {
    let parts: Vec<String> = sections
        .iter()
        .filter_map(|s| {
            let header = s
                .header
                .as_ref()
                .map(|h| {
                    let text = h.text.trim();
                    format!("**{text}**")
                })
                .filter(|h| !h.is_empty());
            let body = format_text_view_model(&s.body);
            if header.is_none() && body.is_empty() {
                return None;
            }
            Some(match header {
                Some(h) => format!("{h}\n\n{body}"),
                None => body,
            })
        })
        .collect();
    parts.join("\n\n---\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_fixture() -> DescriptionSection {
        let json = include_str!("fixtures/job_4439699193.json");
        serde_json::from_str(json).expect("fixture is valid JSON")
    }

    #[test]
    fn preserves_all_text() {
        let section = load_fixture();
        let md = format_text_view_model(&section.body);
        let raw_text = &section.body.text;

        // The Markdown version should be longer (or equal) because it adds markers and bullets.
        assert!(
            md.len() >= raw_text.len(),
            "Markdown length {} should be at least raw text length {}",
            md.len(),
            raw_text.len()
        );

        // Check key sections are present.
        assert!(md.contains("**Own Every Moment at NetApp**"));
        assert!(md.contains("**Essential Functions**"));
        assert!(md.contains("Job Requirements"));
        assert!(md.contains("- Analyzing and defining software and firmware requirements."));
    }

    #[test]
    fn header_is_included() {
        let section = load_fixture();
        let md = description_sections_to_markdown(&[section]);
        assert!(md.starts_with("**About the job**"));
    }

    #[test]
    fn list_items_preserved() {
        let section = load_fixture();
        let md = format_text_view_model(&section.body);
        let list_items = md.matches("- ").count();
        assert!(
            list_items >= 30,
            "expected at least 30 list items, got {list_items}"
        );
    }

    #[test]
    fn handles_inline_styles_and_links() {
        let tv = TextViewModel {
            text: "Visit our website for details.".to_string(),
            attributes: vec![
                TextAttribute {
                    start: 0,
                    length: 5,
                    detail_data: TextAttributeDetail {
                        style: Some("BOLD".to_string()),
                        ..Default::default()
                    },
                },
                TextAttribute {
                    start: 10,
                    length: 7,
                    detail_data: TextAttributeDetail {
                        style: Some("URL".to_string()),
                        hyperlink: Some("https://example.com".to_string()),
                        ..Default::default()
                    },
                },
            ],
        };
        let md = format_text_view_model(&tv);
        assert!(md.contains("**Visit**"));
        assert!(md.contains("[website](https://example.com)"));
    }

    #[test]
    fn list_items_converted_from_bullets() {
        let tv = TextViewModel {
            text: "Item one\n• Item two".to_string(),
            attributes: vec![TextAttribute {
                start: 10,
                length: 9,
                detail_data: TextAttributeDetail {
                    style: Some("LIST_ITEM".to_string()),
                    ..Default::default()
                },
            }],
        };
        let md = format_text_view_model(&tv);
        assert!(md.contains("- Item two"));
    }

    #[test]
    fn empty_text_returns_empty() {
        let tv = TextViewModel {
            text: String::new(),
            attributes: vec![],
        };
        assert!(format_text_view_model(&tv).is_empty());
    }
}
