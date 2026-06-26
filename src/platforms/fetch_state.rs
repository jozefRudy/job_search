#[derive(Debug, Default, Clone, Copy)]
pub struct FetchState {
    new: usize,
    existing: usize,
}

impl FetchState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inc_new(&mut self) {
        self.new += 1;
    }

    pub fn inc_existing(&mut self) {
        self.existing += 1;
    }

    #[must_use]
    pub fn checked(&self) -> usize {
        self.existing + self.new
    }

    #[must_use]
    pub fn new_count(&self) -> usize {
        self.new
    }

    #[must_use]
    pub fn existing(&self) -> usize {
        self.existing
    }

    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Total checked: {} ({} new, {} existing)",
            self.checked(),
            self.new,
            self.existing
        )
    }

    #[must_use]
    pub fn progress_line(&self, total: Option<usize>, item_name: &str) -> String {
        match total {
            Some(t) => format!(
                "\r    Progress: {:>5}/{:<5} {:<40.40}",
                self.checked(),
                t,
                item_name
            ),
            None => format!("\r    Progress: {:>5} {:<40.40}", self.checked(), item_name),
        }
    }
}
