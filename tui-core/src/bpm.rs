use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rand::RngExt as _;

pub mod overlay;

pub const MIN_BPM: f64 = 20.0;
pub const MAX_BPM: f64 = 300.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BpmMode {
    /// 自動モードと、その寄せ先として引いた BPM。
    ///
    /// grid sequencer はこの値をそのままテンポに使い、loop browser は配置 clip の
    /// time stretch 範囲へこの値を clamp する。値を variant に持たせてあるので、
    /// 再抽選は `Auto` 同士でも `PartialEq` で差分として検出できる。
    Auto(f64),
    Manual(f64),
}

impl BpmMode {
    /// 保存済みの手動 BPM から復元する。無効値・未保存なら `auto_bpm` の自動モード。
    pub fn from_saved(manual: Option<f64>, auto_bpm: f64) -> Self {
        manual
            .and_then(valid_bpm)
            .map(Self::Manual)
            .unwrap_or(Self::Auto(auto_bpm))
    }

    pub fn manual(self) -> Option<f64> {
        match self {
            Self::Auto(_) => None,
            Self::Manual(bpm) => Some(bpm),
        }
    }

    /// 自動モードの寄せ先。手動モードなら `None`。
    pub fn auto_target(self) -> Option<f64> {
        match self {
            Self::Auto(bpm) => Some(bpm),
            Self::Manual(_) => None,
        }
    }

    /// このモードが指す BPM。loop browser のように clamp が挟まる画面では
    /// 「clamp 前の希望値」を意味する。
    pub fn bpm(self) -> f64 {
        match self {
            Self::Auto(bpm) | Self::Manual(bpm) => bpm,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto(_) => "AUTO",
            Self::Manual(_) => "MANUAL",
        }
    }
}

/// 自動 BPM を引く範囲。両端は整数 BPM で、`minimum == maximum` なら固定値と同じ。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BpmRange {
    minimum: f64,
    maximum: f64,
}

impl BpmRange {
    pub fn new(minimum: f64, maximum: f64) -> Option<Self> {
        let minimum = integer_bpm(minimum)?;
        let maximum = integer_bpm(maximum)?;
        (minimum <= maximum).then_some(Self { minimum, maximum })
    }

    /// 幅を持たない範囲。範囲を設定するまでの既定値に使う。
    pub fn fixed(bpm: f64) -> Self {
        Self {
            minimum: bpm,
            maximum: bpm,
        }
    }

    pub fn minimum(self) -> f64 {
        self.minimum
    }

    pub fn maximum(self) -> f64 {
        self.maximum
    }

    /// 幅を持つか。持たないなら抽選しても値が動かない＝再抽選を省ける。
    pub fn is_fixed(self) -> bool {
        self.minimum >= self.maximum
    }

    /// 範囲内から整数 BPM を一様に引く。
    ///
    /// 小数で引かないのは、ステータス行に `133.28471…` のような値が出るのを避けるため。
    pub fn sample(self) -> f64 {
        if self.is_fixed() {
            return self.minimum;
        }
        let minimum = self.minimum as i64;
        let maximum = self.maximum as i64;
        rand::rng().random_range(minimum..=maximum) as f64
    }

    pub fn label(self) -> String {
        if self.is_fixed() {
            format_bpm(self.minimum)
        } else {
            format!("{}〜{}", format_bpm(self.minimum), format_bpm(self.maximum))
        }
    }
}

pub fn valid_bpm(bpm: f64) -> Option<f64> {
    (bpm.is_finite() && (MIN_BPM..=MAX_BPM).contains(&bpm)).then_some(bpm)
}

/// 範囲の端として使える整数 BPM だけを通す。
fn integer_bpm(bpm: f64) -> Option<f64> {
    valid_bpm(bpm).filter(|bpm| bpm.fract() == 0.0)
}

/// BPM を表示用の文字列にする。整数なら小数部を出さず、小数は2桁までで末尾0を落とす。
pub fn format_bpm(bpm: f64) -> String {
    let rounded = bpm.round();
    if (bpm - rounded).abs() < 0.000_000_1 {
        format!("{rounded:.0}")
    } else {
        let formatted = format!("{bpm:.2}");
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
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
    /// 単発の数値を入力して確定した。
    Apply(BpmMode),
    /// 自動モードへ入る。`Some` なら範囲も更新する（`None` は現在の範囲のまま引き直す）。
    ApplyAuto(Option<BpmRange>),
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
                BpmInputAction::ApplyAuto(None)
            }
            KeyCode::Enter => match self.parse() {
                Ok(action) => action,
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
                if self.accepts(character) {
                    self.buffer.push(character);
                }
                BpmInputAction::Continue
            }
            _ => BpmInputAction::Continue,
        }
    }

    /// 入力できる文字かどうか。範囲は整数どうしなので、`-` と `.` は同居させない。
    fn accepts(&self, character: char) -> bool {
        match character {
            '0'..='9' => true,
            '.' => !self.buffer.contains('.') && !self.buffer.contains('-'),
            '-' => {
                !self.buffer.is_empty() && !self.buffer.contains('-') && !self.buffer.contains('.')
            }
            _ => false,
        }
    }

    fn parse(&self) -> Result<BpmInputAction, String> {
        let Some((minimum, maximum)) = self.buffer.split_once('-') else {
            let bpm = self
                .buffer
                .parse::<f64>()
                .map_err(|_| "BPMを数値で入力してください".to_string())?;
            return Ok(BpmInputAction::Apply(BpmMode::Manual(out_of_range(bpm)?)));
        };
        let minimum = parse_range_end(minimum)?;
        let maximum = parse_range_end(maximum)?;
        BpmRange::new(minimum, maximum)
            .map(|range| BpmInputAction::ApplyAuto(Some(range)))
            .ok_or_else(|| "自動BPMの範囲は小さい方を先に入力してください".to_string())
    }
}

fn parse_range_end(text: &str) -> Result<f64, String> {
    let bpm = text
        .parse::<f64>()
        .map_err(|_| "自動BPMの範囲は「80-160」の形式で入力してください".to_string())?;
    out_of_range(bpm)
}

fn out_of_range(bpm: f64) -> Result<f64, String> {
    valid_bpm(bpm).ok_or_else(|| format!("BPMは{MIN_BPM:.0}〜{MAX_BPM:.0}で入力してください"))
}

#[cfg(test)]
mod tests;
