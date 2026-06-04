use crate::models::{Data, Job};
use comfy_table::{Cell, CellAlignment, ContentArrangement, Table, presets::UTF8_FULL};

/// Render HTML to plain text suitable for terminal display.
fn html_to_text(html: &str) -> String {
    let text = html2text::from_read(html.as_bytes(), 80).unwrap_or_else(|_| html.to_string());
    text.lines()
        .filter(|l| !l.trim().chars().all(|c| c == '#'))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn fmt_relative(dt: Option<chrono::DateTime<chrono::Utc>>) -> String {
    let dt = match dt {
        Some(d) => d,
        None => return "?".to_string(),
    };
    let dur = chrono::Utc::now().signed_duration_since(dt);
    let mins = dur.num_minutes();
    if mins < 1 {
        return "just now".to_string();
    }
    if mins < 60 {
        return format!("{}m ago", mins);
    }
    let hrs = dur.num_hours();
    if hrs < 24 {
        return format!("{}h ago", hrs);
    }
    let days = dur.num_days();
    if days < 7 {
        return format!("{}d ago", days);
    }
    format!("{}w ago", days / 7)
}

pub fn render_table(jobs: &[Job]) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Id", "Platform", "Posted", "Budget", "Title"]);

    for job in jobs {
        table.add_row(vec![
            Cell::new(job.id.unwrap_or(0)),
            Cell::new(job.platform.to_string()),
            Cell::new(fmt_relative(job.created_at)),
            Cell::new(job.budget.as_deref().unwrap_or("?")),
            Cell::new(&job.title),
        ]);
    }

    if let Some(column) = table.column_mut(0) {
        column.set_cell_alignment(CellAlignment::Right);
    }

    table.to_string()
}

pub fn render_job_detailed(job: &Job) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "#{} [{}] {} | {} | {}",
        job.id.unwrap_or(0),
        job.platform,
        fmt_relative(job.created_at),
        job.budget.as_deref().unwrap_or("?"),
        job.title
    ));
    if !job.tags.is_empty() {
        lines.push(format!("  Tags: {}", job.tags.join(", ")));
    }
    lines.push(format!("  URL:  {}", job.url));

    match &job.raw {
        Data::Upwork { detail } => {
            if !detail.exact_budget.is_empty() {
                lines.push(format!("  Exact budget:   {}", detail.exact_budget));
            }
            if !detail.experience_level.is_empty() {
                lines.push(format!("  Experience:     {}", detail.experience_level));
            }
            if !detail.project_type.is_empty() {
                lines.push(format!("  Project type:   {}", detail.project_type));
            }
            if !detail.duration.is_empty() {
                lines.push(format!("  Duration:       {}", detail.duration));
            }
            if !detail.hours_per_week.is_empty() {
                lines.push(format!("  Hours/week:     {}", detail.hours_per_week));
            }
            if !detail.hires.is_empty() {
                lines.push(format!("  Hires:          {}", detail.hires));
            }
            if !detail.proposals.is_empty() {
                lines.push(format!("  Proposals:      {}", detail.proposals));
            }
            if !detail.last_viewed.is_empty() {
                lines.push(format!("  Last viewed:    {}", detail.last_viewed));
            }
            if !detail.interviewing.is_empty() {
                lines.push(format!("  Interviewing:   {}", detail.interviewing));
            }
            if !detail.invites_sent.is_empty() {
                lines.push(format!("  Invites sent:   {}", detail.invites_sent));
            }
            if !detail.unanswered_invites.is_empty() {
                lines.push(format!("  Unanswered:     {}", detail.unanswered_invites));
            }
            if !detail.description.is_empty() {
                lines.push(format!(
                    "  Description:\n    {}",
                    detail.description.replace('\n', "\n    ")
                ));
            }
        }
        Data::Nofluffjobs { detail } => {
            if !detail.company.is_empty() {
                lines.push(format!("  Company:        {}", detail.company));
            }
            if !detail.seniority.is_empty() {
                lines.push(format!("  Seniority:      {}", detail.seniority));
            }
            if !detail.remote.is_empty() {
                lines.push(format!("  Remote:         {}", detail.remote));
            }
            if !detail.locations.is_empty() {
                lines.push(format!("  Locations:      {}", detail.locations.join(", ")));
            }
            if !detail.offer_valid_until.is_empty() {
                lines.push(format!("  Valid until:    {}", detail.offer_valid_until));
            }
            if !detail.must_have.is_empty() {
                lines.push(format!("  Must have:      {}", detail.must_have.join(", ")));
            }
            if !detail.languages.is_empty() {
                lines.push(format!("  Languages:      {}", detail.languages.join(", ")));
            }
            if !detail.requirements.is_empty() {
                let text = html_to_text(&detail.requirements)
                    .lines()
                    .map(|l| l.trim_end())
                    .collect::<Vec<_>>()
                    .join("\n    ");
                lines.push(format!("  Requirements:\n    {}", text));
            }
            if !detail.offer_description.is_empty() {
                let text = html_to_text(&detail.offer_description)
                    .lines()
                    .map(|l| l.trim_end())
                    .collect::<Vec<_>>()
                    .join("\n    ");
                lines.push(format!("  Offer desc:\n    {}", text));
            }
        }
    }

    lines.push("─".repeat(60));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_fmt_relative_just_now() {
        assert_eq!(fmt_relative(Some(Utc::now())), "just now");
    }

    #[test]
    fn test_fmt_relative_minutes() {
        let dt = Utc::now() - Duration::minutes(5);
        assert_eq!(fmt_relative(Some(dt)), "5m ago");
    }

    #[test]
    fn test_fmt_relative_hours() {
        let dt = Utc::now() - Duration::hours(3);
        assert_eq!(fmt_relative(Some(dt)), "3h ago");
    }

    #[test]
    fn test_fmt_relative_days() {
        let dt = Utc::now() - Duration::days(2);
        assert_eq!(fmt_relative(Some(dt)), "2d ago");
    }

    #[test]
    fn test_fmt_relative_weeks() {
        let dt = Utc::now() - Duration::days(21);
        assert_eq!(fmt_relative(Some(dt)), "3w ago");
    }

    #[test]
    fn test_fmt_relative_none() {
        assert_eq!(fmt_relative(None), "?");
    }
}
