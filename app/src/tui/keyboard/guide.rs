use std::time::{Duration, Instant};

const GUIDE_DELAY: Duration = Duration::from_secs(1);

pub(in crate::tui) const KEYBOARD_NOTE_GUIDE_MESSAGE: &str =
    "c,d,e,f,g,a,bキーを押して音が鳴ることを確認してください";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::tui) enum KeyboardNoteGuidePresentation {
    #[default]
    Hidden,
    Overlay,
    Footer,
}

pub(crate) struct KeyboardNoteGuide {
    last_overlay_date: Option<String>,
    ready_since: Option<Instant>,
    completed: bool,
    presentation: KeyboardNoteGuidePresentation,
}

impl KeyboardNoteGuide {
    pub(in crate::tui) fn new(last_overlay_date: Option<String>) -> Self {
        Self {
            last_overlay_date: last_overlay_date.filter(|date| is_valid_date(date)),
            ready_since: None,
            completed: false,
            presentation: KeyboardNoteGuidePresentation::Hidden,
        }
    }

    pub(super) fn reset_for_screen(&mut self) {
        self.ready_since = None;
        self.completed = false;
        self.presentation = KeyboardNoteGuidePresentation::Hidden;
    }

    /// Ready の継続時間を進める。日次 overlay を新たに表示した場合だけ true を返す。
    pub(super) fn tick(&mut self, now: Instant, ready: bool, local_date: &str) -> bool {
        if self.completed || self.presentation != KeyboardNoteGuidePresentation::Hidden {
            return false;
        }
        if !ready {
            self.ready_since = None;
            return false;
        }
        let ready_since = self.ready_since.get_or_insert(now);
        if now.saturating_duration_since(*ready_since) < GUIDE_DELAY {
            return false;
        }

        if self.last_overlay_date.as_deref() == Some(local_date) {
            self.presentation = KeyboardNoteGuidePresentation::Footer;
            false
        } else {
            self.last_overlay_date = Some(local_date.to_owned());
            self.presentation = KeyboardNoteGuidePresentation::Overlay;
            true
        }
    }

    pub(super) fn complete_note_check(&mut self) {
        self.completed = true;
        self.ready_since = None;
        self.presentation = KeyboardNoteGuidePresentation::Hidden;
    }

    pub(in crate::tui) fn presentation(&self) -> KeyboardNoteGuidePresentation {
        self.presentation
    }

    pub(in crate::tui) fn last_overlay_date(&self) -> Option<&str> {
        self.last_overlay_date.as_deref()
    }
}

pub(super) fn local_date_string() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn is_valid_date(date: &str) -> bool {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_waits_one_second_of_ready_time() {
        let start = Instant::now();
        let mut guide = KeyboardNoteGuide::new(None);

        assert!(!guide.tick(start, true, "2026-07-20"));
        assert!(!guide.tick(start + Duration::from_millis(999), true, "2026-07-20"));
        assert_eq!(guide.presentation(), KeyboardNoteGuidePresentation::Hidden);
        assert!(guide.tick(start + GUIDE_DELAY, true, "2026-07-20"));
        assert_eq!(guide.presentation(), KeyboardNoteGuidePresentation::Overlay);
    }

    #[test]
    fn non_ready_time_does_not_count_toward_delay() {
        let start = Instant::now();
        let mut guide = KeyboardNoteGuide::new(None);

        assert!(!guide.tick(start, true, "2026-07-20"));
        assert!(!guide.tick(start + GUIDE_DELAY, false, "2026-07-20"));
        assert!(!guide.tick(
            start + GUIDE_DELAY + Duration::from_millis(999),
            true,
            "2026-07-20"
        ));
        assert!(guide.tick(
            start + GUIDE_DELAY * 2 + Duration::from_millis(999),
            true,
            "2026-07-20"
        ));
    }

    #[test]
    fn same_date_uses_footer_and_next_date_uses_overlay() {
        let start = Instant::now();
        let mut guide = KeyboardNoteGuide::new(Some("2026-07-20".to_string()));

        assert!(!guide.tick(start, true, "2026-07-20"));
        assert!(!guide.tick(start + GUIDE_DELAY, true, "2026-07-20"));
        assert_eq!(guide.presentation(), KeyboardNoteGuidePresentation::Footer);

        guide.reset_for_screen();
        assert!(!guide.tick(start, true, "2026-07-21"));
        assert!(guide.tick(start + GUIDE_DELAY, true, "2026-07-21"));
        assert_eq!(guide.presentation(), KeyboardNoteGuidePresentation::Overlay);
    }

    #[test]
    fn successful_note_hides_prompt_and_prevents_it_for_current_screen() {
        let start = Instant::now();
        let mut guide = KeyboardNoteGuide::new(None);
        assert!(!guide.tick(start, true, "2026-07-20"));

        guide.complete_note_check();

        assert!(!guide.tick(start + GUIDE_DELAY, true, "2026-07-20"));
        assert_eq!(guide.presentation(), KeyboardNoteGuidePresentation::Hidden);
    }

    #[test]
    fn invalid_saved_date_is_treated_as_never_shown() {
        let start = Instant::now();
        let mut guide = KeyboardNoteGuide::new(Some("not-a-date".to_string()));

        assert!(!guide.tick(start, true, "2026-07-20"));
        assert!(guide.tick(start + GUIDE_DELAY, true, "2026-07-20"));
        assert_eq!(guide.last_overlay_date(), Some("2026-07-20"));
    }
}
