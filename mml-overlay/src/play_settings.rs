//! `Ctrl+L` で開く演奏設定。
//!
//! repeat / CC1 modulation / velocity の 3 値を持つ。**設定は MML overlay 全体で共通**で、
//! 音色選択を開いている最中にも同じ設定が効く。そのため開くキーは音色選択より手前で拾い、
//! このモーダルは overlay の最も手前に立つ。
//!
//! 「いまの値」（[`PlaySettings`]）と「編集中のモーダル」（[`PlaySettingsSelect`]）を分ける。
//! モーダルは開いた時点の値を握っていて、取り消しはそこへ丸ごと巻き戻す。項目ごとに
//! 巻き戻すのではなく丸ごと戻すので、複数項目を触ってから Esc しても取りこぼさない。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::line_play::{FilterSettings, LinePerformance, LineProgram};

/// MML overlay 全体で共通の演奏設定。
///
/// [`crate::line_play::LineProgram`] へそのまま載る形にしてある（`repeat` と `filters` が
/// `LineProgram` の同名フィールドに対応する）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlaySettings {
    /// 鳴らし終わっても止めず、同じ内容を継ぎ足して鳴らし続ける。
    pub repeat: bool,
    /// 演奏へ重ねる MIDI filter。
    pub filters: FilterSettings,
}

impl PlaySettings {
    /// この設定でこの演奏を鳴らす指示にする。
    ///
    /// **設定を音へ効かせる唯一の合流点。** 行を積む経路（`Ctrl+Space` / 行の移動 /
    /// 履歴の試聴）はどれもここを通すので、「設定は overlay 全体で共通」が
    /// 経路ごとの取りこぼしなしに成り立つ。設定を持たない呼び出し側だけが
    /// [`LineProgram::once`] を使う。
    pub(crate) fn program(self, performance: LinePerformance) -> LineProgram {
        LineProgram {
            performance,
            repeat: self.repeat,
            filters: self.filters,
        }
    }
}

/// 演奏設定の項目。表示順もカーソル位置もこの並びで決まる。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlaySettingsItem {
    Repeat,
    Modulation,
    Velocity,
}

impl PlaySettingsItem {
    pub(crate) const ALL: [Self; 3] = [Self::Repeat, Self::Modulation, Self::Velocity];

    /// 項目名。
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Repeat => "repeat",
            Self::Modulation => "CC1 modulation",
            Self::Velocity => "velocity",
        }
    }

    /// 項目名の右に出す補足。その項目が何をするのかを 1 行で言う。
    pub(crate) fn detail(self) -> &'static str {
        match self {
            Self::Repeat => "行をつなげて鳴らし続ける",
            Self::Modulation | Self::Velocity => "0→127→0 / 4秒",
        }
    }

    pub(crate) fn is_on(self, settings: &PlaySettings) -> bool {
        match self {
            Self::Repeat => settings.repeat,
            Self::Modulation => settings.filters.modulation,
            Self::Velocity => settings.filters.velocity,
        }
    }

    fn toggle(self, settings: &mut PlaySettings) {
        match self {
            Self::Repeat => settings.repeat = !settings.repeat,
            Self::Modulation => settings.filters.modulation = !settings.filters.modulation,
            Self::Velocity => settings.filters.velocity = !settings.filters.velocity,
        }
    }
}

/// 演奏設定モーダルが呼び出し側へ求める処理。
pub(crate) enum PlaySettingsAction {
    /// 表示が変わっただけ。まだ閉じない。
    Continue,
    /// 確定して閉じる。載っているのは編集後の値。
    Confirm(PlaySettings),
    /// 取り消して閉じる。載っているのは開いた時点の値。
    Cancel(PlaySettings),
}

/// 編集中の演奏設定モーダル。
pub(crate) struct PlaySettingsSelect {
    /// 開いた時点の値。取り消しで戻す先。
    original: PlaySettings,
    /// 編集中の値。確定するとこれが採用される。
    current: PlaySettings,
    cursor: usize,
}

impl PlaySettingsSelect {
    pub(crate) fn open(settings: PlaySettings) -> Self {
        Self {
            original: settings,
            current: settings,
            cursor: 0,
        }
    }

    /// 編集中の値（描画用）。
    pub(crate) fn settings(&self) -> &PlaySettings {
        &self.current
    }

    /// 選択中の項目（描画用）。
    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> PlaySettingsAction {
        // 開くキーをもう一度押したら閉じる。開閉が同じキーなら、開けたことに気づいて
        // から抜ける手段を別に覚えずに済む。
        if is_play_settings_trigger(key) {
            return PlaySettingsAction::Cancel(self.original);
        }
        if is_cancel_key(key) {
            return PlaySettingsAction::Cancel(self.original);
        }
        match key.code {
            KeyCode::Enter => PlaySettingsAction::Confirm(self.current),
            KeyCode::Up => self.move_cursor(-1),
            KeyCode::Down => self.move_cursor(1),
            KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') => self.toggle_selected(),
            _ => PlaySettingsAction::Continue,
        }
    }

    fn move_cursor(&mut self, delta: isize) -> PlaySettingsAction {
        let last = PlaySettingsItem::ALL.len() - 1;
        self.cursor = self.cursor.saturating_add_signed(delta).min(last);
        PlaySettingsAction::Continue
    }

    fn toggle_selected(&mut self) -> PlaySettingsAction {
        if let Some(item) = PlaySettingsItem::ALL.get(self.cursor) {
            item.toggle(&mut self.current);
        }
        PlaySettingsAction::Continue
    }
}

/// このキーは取り消して閉じる。
///
/// `Q` は入力欄の打鍵ではなくモーダルの外し方として拾う（モーダル中はどのみち
/// 入力欄へ文字は入らない）。
fn is_cancel_key(key: KeyEvent) -> bool {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return false;
    }
    matches!(
        key.code,
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q')
    )
}

/// このキーは演奏設定を開閉する。
///
/// overlay 本体からも音色選択からも同じキーで開く（設定は overlay 全体で共通のため）。
/// `Ctrl` + a〜z のうち、入力欄の textarea と既存の overlay キーが取っていないのは
/// `g` / `l` / `q` / `s` / `z` だけ。Loop の頭文字を取って `l` を使う。
pub fn is_play_settings_trigger(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('l')
}

#[cfg(test)]
mod tests;
