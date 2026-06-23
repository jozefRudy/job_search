/// RAII guard for terminal cursor visibility.
/// Hides cursor on creation, shows on drop (even on early return).
pub struct CursorGuard;

impl CursorGuard {
    pub fn new() -> Self {
        hide_cursor();
        Self
    }
}

impl Default for CursorGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CursorGuard {
    fn drop(&mut self) {
        show_cursor();
        new_line();
    }
}

/// Hide terminal cursor (ANSI escape).
fn hide_cursor() {
    eprint!("\x1B[?25l");
}

/// Show terminal cursor (ANSI escape).
fn show_cursor() {
    eprint!("\x1B[?25h");
}

/// Move to next line.
fn new_line() {
    eprintln!();
}
