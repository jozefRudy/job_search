#[derive(Debug, Default, Clone, Copy)]
pub struct FetchState {
    new: usize,
    existing: usize,
    skipped: usize,
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

    pub fn inc_existing_n(&mut self, n: usize) {
        self.existing += n;
    }

    pub fn inc_skipped(&mut self) {
        self.skipped += 1;
    }

    #[must_use]
    pub fn checked(&self) -> usize {
        self.existing + self.new + self.skipped
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
            "Total checked: {} ({} new, {} existing, {} skipped)",
            self.checked(),
            self.new,
            self.existing,
            self.skipped
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

#[cfg(test)]
mod tests {
    use super::FetchState;

    #[test]
    fn skipped_increments_checked_and_summary() {
        let mut state = FetchState::new();
        state.inc_new();
        state.inc_existing();
        state.inc_skipped();
        state.inc_skipped();

        assert_eq!(state.checked(), 4);
        assert_eq!(state.new_count(), 1);
        assert_eq!(state.existing(), 1);
        assert_eq!(
            state.summary(),
            "Total checked: 4 (1 new, 1 existing, 2 skipped)"
        );
    }

    #[test]
    fn progress_line_includes_skipped() {
        let mut state = FetchState::new();
        state.inc_skipped();
        let line = state.progress_line(Some(5), "test");
        assert!(line.contains("1/5"));
    }
}
