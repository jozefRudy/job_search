use crate::models::{Data, Job, Platform};
use comfy_table::{Cell, CellAlignment, ContentArrangement, Table, presets::UTF8_FULL};

pub fn fmt_relative(dt: chrono::DateTime<chrono::Utc>) -> String {
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

fn indent_md(text: &str) -> String {
    text.replace('\n', "\n    ")
}

fn align_columns(table: &mut Table, headers: &[&str], align: &[(&str, CellAlignment)]) {
    for &(name, a) in align {
        let Some(idx) = headers.iter().position(|&h| h == name) else {
            continue;
        };
        table
            .column_mut(idx)
            .expect("header was just set, column must exist")
            .set_cell_alignment(a);
    }
}

pub fn render_table(jobs: &[Job], platform: Option<Platform>) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    match platform {
        None => {
            let headers = [
                "Id", "Platform", "Posted", "Budget", "Applied", "Rating", "Title", "#",
            ];
            table.set_header(headers);
            for (i, job) in jobs.iter().enumerate() {
                table.add_row(vec![
                    Cell::new(job.id.unwrap_or(0)),
                    Cell::new(job.platform.to_string()),
                    Cell::new(fmt_relative(job.created_at)),
                    Cell::new(job.budget.as_deref().unwrap_or("?")),
                    Cell::new(job.applied_at.map_or(String::new(), fmt_relative)),
                    Cell::new(match job.liked {
                        Some(true) => "👍",
                        Some(false) => "👎",
                        None => "",
                    }),
                    Cell::new(&job.title),
                    Cell::new(i + 1),
                ]);
            }
            align_columns(
                &mut table,
                &headers,
                &[("Id", CellAlignment::Right), ("#", CellAlignment::Right)],
            );
        }
        Some(Platform::Upwork) => {
            let headers = [
                "Id",
                "Posted",
                "Budget",
                "Applied",
                "Rating",
                "Last viewed",
                "Title",
                "#",
            ];
            table.set_header(headers);
            for (i, job) in jobs.iter().enumerate() {
                let Data::Upwork { detail } = &job.raw else {
                    unreachable!("upwork table only renders upwork jobs");
                };
                let last_viewed = detail.last_viewed.map(fmt_relative).unwrap_or_default();
                table.add_row(vec![
                    Cell::new(job.id.unwrap_or(0)),
                    Cell::new(fmt_relative(job.created_at)),
                    Cell::new(job.budget.as_deref().unwrap_or("?")),
                    Cell::new(job.applied_at.map_or(String::new(), fmt_relative)),
                    Cell::new(match job.liked {
                        Some(true) => "👍",
                        Some(false) => "👎",
                        None => "",
                    }),
                    Cell::new(last_viewed),
                    Cell::new(&job.title),
                    Cell::new(i + 1),
                ]);
            }
            align_columns(
                &mut table,
                &headers,
                &[("Id", CellAlignment::Right), ("#", CellAlignment::Right)],
            );
        }
        Some(Platform::NoFluffJobs) => {
            let headers = ["Id", "Posted", "Budget", "Applied", "Rating", "Title", "#"];
            table.set_header(headers);
            for (i, job) in jobs.iter().enumerate() {
                table.add_row(vec![
                    Cell::new(job.id.unwrap_or(0)),
                    Cell::new(fmt_relative(job.created_at)),
                    Cell::new(job.budget.as_deref().unwrap_or("?")),
                    Cell::new(job.applied_at.map_or(String::new(), fmt_relative)),
                    Cell::new(match job.liked {
                        Some(true) => "👍",
                        Some(false) => "👎",
                        None => "",
                    }),
                    Cell::new(&job.title),
                    Cell::new(i + 1),
                ]);
            }
            align_columns(
                &mut table,
                &headers,
                &[("Id", CellAlignment::Right), ("#", CellAlignment::Right)],
            );
        }
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
            if let Some(dt) = detail.last_viewed {
                lines.push(format!("  Last viewed by client: {}", fmt_relative(dt)));
            } else {
                lines.push("  Last viewed by client: never".to_string());
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
                    indent_md(&detail.description)
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
                lines.push(format!(
                    "  Requirements:\n    {}",
                    indent_md(&detail.requirements)
                ));
            }
            if !detail.nice_to_have.is_empty() {
                lines.push(format!(
                    "  Nice to have:\n    {}",
                    indent_md(&detail.nice_to_have)
                ));
            }
            if !detail.description.is_empty() {
                lines.push(format!(
                    "  Description:\n    {}",
                    indent_md(&detail.description)
                ));
            }
        }
    }

    lines.push(format!(
        "  Rating:         {}",
        match job.liked {
            Some(true) => "liked",
            Some(false) => "disliked",
            None => "neutral",
        }
    ));
    if let Some(applied) = job.applied_at {
        lines.push(format!("  Applied:        {}", fmt_relative(applied)));
    } else {
        lines.push("  Applied:        no".to_string());
    }
    if let Some(note) = &job.note
        && !note.is_empty()
    {
        lines.push("  Note:".to_string());
        for line in note.lines() {
            lines.push(format!("    {}", line));
        }
    }
    lines.push("".to_string());
    lines.push("─".repeat(60));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_fmt_relative_just_now() {
        assert_eq!(fmt_relative(Utc::now()), "just now");
    }

    #[test]
    fn test_fmt_relative_minutes() {
        let dt = Utc::now() - Duration::minutes(5);
        assert_eq!(fmt_relative(dt), "5m ago");
    }

    #[test]
    fn test_fmt_relative_hours() {
        let dt = Utc::now() - Duration::hours(3);
        assert_eq!(fmt_relative(dt), "3h ago");
    }

    #[test]
    fn test_fmt_relative_days() {
        let dt = Utc::now() - Duration::days(2);
        assert_eq!(fmt_relative(dt), "2d ago");
    }

    #[test]
    fn test_fmt_relative_weeks() {
        let dt = Utc::now() - Duration::days(21);
        assert_eq!(fmt_relative(dt), "3w ago");
    }
}
