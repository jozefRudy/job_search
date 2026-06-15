#[derive(Debug, Default, Clone, Copy)]
pub struct FetchState {
    checked: usize,
    new: usize,
    existing: usize,
}

impl FetchState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inc_checked(&mut self) {
        self.checked += 1;
    }

    pub fn inc_new(&mut self) {
        self.new += 1;
    }

    pub fn inc_existing(&mut self) {
        self.existing += 1;
    }

    pub fn checked(&self) -> usize {
        self.checked
    }

    pub fn new_count(&self) -> usize {
        self.new
    }

    pub fn existing(&self) -> usize {
        self.existing
    }

    pub fn summary(&self) -> String {
        format!(
            "Total checked: {} ({} new, {} existing)",
            self.checked, self.new, self.existing
        )
    }

    pub fn progress_line(&self, total: Option<usize>, label: &str) -> String {
        match total {
            Some(t) => format!("\r    Progress: {:>5}/{:<5} {:.40}", self.checked, t, label),
            None => format!("\r    Progress: {:>5} {:.40}", self.checked, label),
        }
    }
}
