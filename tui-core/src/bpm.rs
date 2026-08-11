use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub const MIN_BPM: f64 = 20.0;
pub const MAX_BPM: f64 = 300.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum BpmMode {
    #[default]
    Auto,
    Manual(f64),
}

impl BpmMode {
    pub fn from_saved_manual(bpm: Option<f64>) -> Self {
        bpm.and_then(valid_bpm)
            .map(Self::Manual)
            .unwrap_or(Self::Auto)
    }

    pub fn manual(self) -> Option<f64> {
        match self {
            Self::Auto => None,
            Self::Manual(bpm) => Some(bpm),
        }
    }

    pub fn resolve(self, automatic_bpm: f64) -> f64 {
        self.manual().unwrap_or(automatic_bpm)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "AUTO",
            Self::Manual(_) => "MANUAL",
        }
    }
}

pub fn valid_bpm(bpm: f64) -> Option<f64> {
    (bpm.is_finite() && (MIN_BPM..=MAX_BPM).contains(&bpm)).then_some(bpm)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BpmInput {
    buffer: String,
    error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BpmInputAction {
    Continue,
    Cancel,
    Apply(BpmMode),
}

impl BpmInput {
    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> BpmInputAction {
        self.error = None;
        match key.code {
            KeyCode::Esc => BpmInputAction::Cancel,
            KeyCode::Char('a' | 'A')
                if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
            {
                BpmInputAction::Apply(BpmMode::Auto)
            }
            KeyCode::Enter => match self.parse() {
                Ok(bpm) => BpmInputAction::Apply(BpmMode::Manual(bpm)),
                Err(error) => {
                    self.error = Some(error);
                    BpmInputAction::Continue
                }
            },
            KeyCode::Backspace => {
                self.buffer.pop();
                BpmInputAction::Continue
            }
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if character.is_ascii_digit() || (character == '.' && !self.buffer.contains('.')) {
                    self.buffer.push(character);
                }
                BpmInputAction::Continue
            }
            _ => BpmInputAction::Continue,
        }
    }

    fn parse(&self) -> Result<f64, String> {
        let bpm = self
            .buffer
            .parse::<f64>()
            .map_err(|_| "BPMを数値で入力してください".to_string())?;
        valid_bpm(bpm).ok_or_else(|| format!("BPMは{MIN_BPM:.0}〜{MAX_BPM:.0}で入力してください"))
    }
}

#[cfg(test)]
mod tests;
