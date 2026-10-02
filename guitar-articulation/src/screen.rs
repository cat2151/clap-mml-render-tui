//! Guitar Articulation 画面の状態とキー処理。
//!
//! 持つのは入力 MML・そこから作ったrawの列・ルール表・Articulatedの列・カーソル列。
//! 音は鳴らさない。入力の取り消し・ヘルプ以外の操作では、鳴らしてほしい版を
//! [`GuitarArticulationAction::Play`] で host へ返す（MML の確定・ルールの toggle はArticulated）。
//! `h` / `l` の移動は、1 音モードに依らずカーソル列の音（[`GuitarArticulationAction::PlayNote`]）を返す。
//! 1 音モード（`n`）の間は、ルールの toggle と `b` / `space` がカーソル列の音だけ
//! （[`GuitarArticulationAction::PlayNote`]）になる。

use cmrt_core::EffectPlugins;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::TextArea;
use serde_json::Value;

use crate::humanize::Humanized;
use crate::ui::{PARAM_LIST_KEY, ROW_RULE_ROWS, RULE_LIST_KEY, RULE_ROWS};
use crate::{
    ArpSettings, Articulated, ColumnRuleAnchor, Instrument, Note, RowRule, Rule, RuleTable,
    StartupInstrument, TimedMidiEvent,
};

/// 画面に入ったとき MML が空なら入れておく MML。何を触れば何が変わるかを、打つ前から見せる。
pub const DEFAULT_MML: &str = "l16cdefgab<c";

/// どちらの版を鳴らすか。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Take {
    /// 入力 MML をそのまま（KS なし）。
    Plain,
    /// ルール表で KS を足した版。
    Converted,
}

impl Take {
    /// ログへ出す綴り。
    pub fn label(self) -> &'static str {
        match self {
            Take::Plain => "plain",
            Take::Converted => "converted",
        }
    }
}

mod arp;
mod arp_materials;
mod arp_rules;
mod columns;
mod effect_chain;
mod history;
mod input;
mod param_list;
mod rule_list;
mod sample_midi;
mod sounding;

pub use arp::ArpRow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuitarArticulationAction {
    Continue,
    Quit,
    /// その版のイベント列（[`GuitarArticulationScreen::events`]）を、確定済みの chain
    /// （[`GuitarArticulationScreen::sounding_effect_chain`]）を掛けて鳴らしてほしい。
    Play(Take),
    /// その版のカーソル列の 1 音（[`crate::column_events`]）を、確定済みの chain で鳴らしてほしい。
    PlayNote {
        take: Take,
        column: usize,
    },
    /// Articulated を、この chain を掛けて試聴してほしい（effect chain overlay の編集中）。
    PreviewEffectChain(Vec<Value>),
    /// 履歴（[`GuitarArticulationScreen::history`]）を file へ書いてほしい。
    SaveHistory,
    /// 画面の設定（[`GuitarArticulationScreen::settings`]）を設定 file へ書いてほしい。
    SaveSettings,
    /// サンプル MID の一覧を [`GuitarArticulationScreen::open_sample_midi_list`] へ渡してほしい。
    OpenSampleMidiList,
    /// この file を読み、[`GuitarArticulationScreen::load_sample_midi`] へ渡してほしい。
    LoadSampleMidi(std::path::PathBuf),
    /// この file を読み、MID モードへは入らずに全体を鳴らしてほしい（一覧で選び直すたびの試聴）。
    PreviewSampleMidi(std::path::PathBuf),
    /// サンプル MID の全体（`None`）か 1 音（[`GuitarArticulationScreen::sample_midi_events`]）を鳴らしてほしい。
    PlaySampleMidi {
        note: Option<usize>,
    },
    /// repeat を OFF にしたので（`Shift+R`、`Shift+R` が OFF のままアルペジエーター overlay を閉じた）、
    /// 繰り返している演奏を止めてほしい。
    StopRepeat,
}

#[derive(Default)]
pub struct GuitarArticulationScreen {
    mml: String,
    /// `mml` が chord 表記として解釈されたか。
    material_from_chord: bool,
    /// アルペジエーターの設定。当てるのは overlay を開いている間だけで、閉じても値は保つ。
    arp: ArpSettings,
    /// アルペジエーター overlay で選ぶ素材（MML / chord）。
    arp_materials: Vec<String>,
    /// overlay で最後に選んだ素材。空ならまだ選んでいない（overlay は `mml` を素材にする）。
    arp_material: String,
    /// 設定 file へ書く値（[`GuitarArticulationScreen::settings`]）を変えてから、まだ書いていないか。
    settings_unsaved: bool,
    /// MML を編集している間だけ `Some`。英字キーを入力欄とルールのどちらへ渡すかをこれで分ける。
    input: Option<TextArea<'static>>,
    plain: Vec<TimedMidiEvent>,
    notes: Vec<Note>,
    rules: RuleTable,
    /// 列ルールを最後に手で切り替えた直後の MML と列ルール。MML の確定で列ルールを付け替える元。
    anchor: Option<ColumnRuleAnchor>,
    /// `notes` と同じ並びの、音ごとの奏法と強さ。
    articulated: Vec<Articulated>,
    converted: Vec<TimedMidiEvent>,
    /// 汚し（[`RowRule::Humanize`]）が ON の間だけ、`notes` と同じ並びの汚しの値（`convert` と同じ計算）。
    humanized: Vec<Humanized>,
    cursor: usize,
    /// 1 音モード。ON の間は演奏の要求がカーソル列の音だけになる。
    note_preview: bool,
    /// repeat（`Shift+R`）。ON の間、host は演奏を鳴らし終わるたびに同じものを鳴らし直す。
    repeat: bool,
    /// 全体の演奏が鳴っている間だけ、その版と演奏の頭からの秒。host が音源の状態から決める。
    playhead: Option<(Take, f64)>,
    help_open: bool,
    effect_plugins: EffectPlugins,
    effect_chain: Vec<Value>,
    /// dry（`w`）。ON の間は `effect_chain` を残したまま、掛けずに鳴らす。
    effect_dry: bool,
    /// effect chain overlay（`x`）を開いている間だけ `Some`。
    effect_overlay: Option<effect_chain::EffectOverlay>,
    /// アルペジエーター overlay を閉じたとき履歴へ積んでから、まだ file へ書いていないか。
    history_unsaved: bool,
    /// 状態が変わるたびに今の状態を積む、新しい順の履歴。
    history: crate::history::GuitarArticulationHistory,
    /// history overlay（`Shift+H`）を開いている間だけ `Some`。
    history_overlay: Option<history::HistoryOverlay>,
    /// いま鳴らす音色の段階。host が音源の状態から決める。
    instrument: Instrument,
    /// 画面に入ったとき最初に読む版（`f`）。次に入ったときから効く。
    startup_instrument: StartupInstrument,
    /// 音色を読み込み中か（先読みを含む）。host が音源の状態から決める。
    sound_loading: bool,
    /// サンプル MID を開いている間だけ `Some`。MML 側の状態には触らない。
    sample_midi: Option<sample_midi::SampleMidiState>,
    /// サンプル MID の一覧 overlay（`o`）を開いている間だけ `Some`。
    sample_midi_list: Option<sample_midi::SampleMidiList>,
    /// 奏法リスト overlay（`t`）を開いている間だけ `Some`。
    rule_list: Option<rule_list::RuleList>,
    /// パラメータ overlay（`u`）を開いている間だけ、選んでいる行。
    param_list: Option<usize>,
    /// アルペジエーター overlay（`z`）を開いている間だけ `Some`。
    arp_overlay: Option<arp::ArpOverlay>,
    /// 直前の操作ができなかった理由。次のキーで消える。
    pub error: Option<String>,
}

impl GuitarArticulationScreen {
    /// `effect_plugins` の catalog から、effect chain overlay（`x`）の候補を出す。
    pub fn with_effect_plugins(effect_plugins: EffectPlugins) -> Self {
        Self {
            effect_plugins,
            ..Self::default()
        }
    }

    pub fn mml(&self) -> &str {
        &self.mml
    }

    pub fn input_open(&self) -> bool {
        self.input.is_some()
    }

    /// 点滅する縦線カーソルを置く入力欄（MML 欄か、effect の list の絞り込み欄）にキーが入る状態か。
    pub fn uses_textarea_cursor(&self) -> bool {
        self.input_open() || self.effect_filter_active() || self.rule_list_filter_input_active()
    }

    pub fn help_open(&self) -> bool {
        self.help_open
    }

    pub(crate) fn input(&self) -> Option<&TextArea<'static>> {
        self.input.as_ref()
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    pub fn rules(&self) -> &RuleTable {
        &self.rules
    }

    pub fn articulated(&self) -> &[Articulated] {
        &self.articulated
    }

    /// カーソルのある列番号。
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// 1 音モードか。
    pub fn note_preview(&self) -> bool {
        self.note_preview
    }

    /// repeat が ON か。`Shift+R` の ON に加え、アルペジエーター overlay（`z`）を開いている間も ON。
    pub fn repeat(&self) -> bool {
        self.repeat || self.arp_overlay.is_some()
    }

    pub fn instrument(&self) -> Instrument {
        self.instrument
    }

    pub fn set_instrument(&mut self, instrument: Instrument) {
        self.instrument = instrument;
    }

    /// 保存済みの起動時の版を持たせる。
    pub fn with_startup_instrument(mut self, startup_instrument: StartupInstrument) -> Self {
        self.startup_instrument = startup_instrument;
        self
    }

    pub fn startup_instrument(&self) -> StartupInstrument {
        self.startup_instrument
    }

    pub fn set_sound_loading(&mut self, loading: bool) {
        self.sound_loading = loading;
    }

    pub fn events(&self, take: Take) -> &[TimedMidiEvent] {
        match take {
            Take::Plain => &self.plain,
            Take::Converted => &self.converted,
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        // ヘルプを開いている間は閉じるキーだけを見る。
        if self.help_open {
            if is_help_key(key) || key.code == KeyCode::Esc {
                self.help_open = false;
            }
            return GuitarArticulationAction::Continue;
        }
        // overlay の絞り込みは正規表現で `?` を打てるので、`?` の扱いも overlay に任せる。
        if self.effect_overlay.is_some() {
            return self.handle_effect_key(key);
        }
        if self.history_overlay.is_some() {
            return self.handle_history_key(key);
        }
        if self.sample_midi_list.is_some() {
            return self.handle_sample_midi_list_key(key);
        }
        if self.rule_list.is_some() {
            return self.handle_rule_list_key(key);
        }
        if self.param_list.is_some() {
            return self.handle_param_list_key(key);
        }
        if self.arp_overlay.is_some() {
            return self.handle_arp_overlay_key(key);
        }
        // MML は `?` を使わないので、入力欄を開いていても `?` はヘルプへ回す。
        if is_help_key(key) {
            self.help_open = true;
            return GuitarArticulationAction::Continue;
        }
        if self.input.is_some() {
            return self.handle_input_key(key);
        }
        self.error = None;
        if self.sample_midi.is_some() {
            return self.handle_sample_midi_key(key);
        }
        if history::is_history_key(key) {
            return self.open_history_overlay();
        }
        if is_shift_key(key, 'R') {
            return self.toggle_repeat();
        }
        if is_shift_key(key, 'A') {
            return self.cycle_accent_pattern();
        }
        match key.code {
            KeyCode::Char('i') => {
                self.open_input();
                GuitarArticulationAction::Continue
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                self.play_cursor_note()
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.cursor + 1 < self.column_count() {
                    self.cursor += 1;
                }
                self.play_cursor_note()
            }
            KeyCode::Char('o') => GuitarArticulationAction::OpenSampleMidiList,
            KeyCode::Char('x') => self.open_effect_overlay(),
            KeyCode::Char('w') => self.toggle_effect_dry(),
            KeyCode::Char('n') => {
                self.note_preview = !self.note_preview;
                GuitarArticulationAction::Continue
            }
            KeyCode::Char('f') => {
                self.startup_instrument = self.startup_instrument.toggled();
                GuitarArticulationAction::SaveSettings
            }
            KeyCode::Char('b') => self.play(Take::Plain),
            KeyCode::Char(' ') => self.play(Take::Converted),
            KeyCode::Char('q') => GuitarArticulationAction::Quit,
            KeyCode::Char(RULE_LIST_KEY) => self.open_rule_list(),
            KeyCode::Char(PARAM_LIST_KEY) => self.open_param_list(),
            KeyCode::Char('z') => self.open_arp_overlay(),
            KeyCode::Char(ch) => {
                if let Some(rule_row) = RULE_ROWS.iter().find(|rule_row| rule_row.key == ch) {
                    self.toggle_rule(rule_row.rule)
                } else if let Some((rule, _, _)) =
                    ROW_RULE_ROWS.iter().find(|(_, key, _)| *key == ch)
                {
                    self.toggle_row_rule(*rule)
                } else {
                    GuitarArticulationAction::Continue
                }
            }
            _ => GuitarArticulationAction::Continue,
        }
    }

    /// カーソル列のルールを切り替え、Articulatedを作り直して鳴らす。
    fn toggle_rule(&mut self, rule: Rule) -> GuitarArticulationAction {
        if self.column_count() == 0 {
            self.error = Some("i で MML を入力してください".to_string());
            return GuitarArticulationAction::Continue;
        }
        self.rules.toggle(self.cursor, rule);
        self.anchor = Some(ColumnRuleAnchor::new(&self.mml, &self.rules));
        self.rebuild_converted();
        self.record_history();
        self.play(Take::Converted)
    }

    /// 行全体のルールを切り替え、Articulatedを作り直して鳴らす。
    /// 自動ハンマリング（`s`）だけは off → on1 → on2 と回す。
    fn toggle_row_rule(&mut self, rule: RowRule) -> GuitarArticulationAction {
        if rule == RowRule::AutoHammerPull {
            self.rules.cycle_auto_hammer_pull();
        } else {
            self.rules.toggle_row(rule);
        }
        self.rebuild_converted();
        self.record_history();
        self.play(Take::Converted)
    }

    /// repeat を切り替える。ON にしたら繰り返し始め、OFF にしたら止める。
    fn toggle_repeat(&mut self) -> GuitarArticulationAction {
        self.repeat = !self.repeat;
        if self.repeat {
            self.play(Take::Converted)
        } else {
            GuitarArticulationAction::StopRepeat
        }
    }

    /// アクセントの選び方（上だけ → 下だけ → 両方）を回し、Articulatedを作り直して鳴らす。
    fn cycle_accent_pattern(&mut self) -> GuitarArticulationAction {
        self.rules.cycle_accent_pattern();
        self.rebuild_converted();
        self.record_history();
        self.play(Take::Converted)
    }

    /// dry と wet を入れ替えて鳴らす。chain を替えた音は、server が読み込み中の音色を
    /// 読み終えるまで用意できないので、読み込み中は入れ替えずに理由を出す。
    fn toggle_effect_dry(&mut self) -> GuitarArticulationAction {
        if self.sound_loading {
            self.error = Some("音色の読み込み中は dry/wet を切り替えられません".to_string());
            return GuitarArticulationAction::Continue;
        }
        self.effect_dry = !self.effect_dry;
        self.play(Take::Converted)
    }

    /// 1 音モードならカーソル列の音だけ、でなければフレーズ全体の演奏を求める。
    fn play(&mut self, take: Take) -> GuitarArticulationAction {
        if self.plain.is_empty() {
            self.error = Some("i で MML を入力してください".to_string());
            return GuitarArticulationAction::Continue;
        }
        if self.note_preview {
            GuitarArticulationAction::PlayNote {
                take,
                column: self.cursor,
            }
        } else {
            GuitarArticulationAction::Play(take)
        }
    }

    /// 1 音モードに依らず、カーソル列の音を Articulated で鳴らす（`h` / `l` の移動のたび）。
    fn play_cursor_note(&self) -> GuitarArticulationAction {
        if self.plain.is_empty() {
            return GuitarArticulationAction::Continue;
        }
        GuitarArticulationAction::PlayNote {
            take: Take::Converted,
            column: self.cursor,
        }
    }
}

fn is_shift_key(key: KeyEvent, upper: char) -> bool {
    key.code == KeyCode::Char(upper)
        && !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

fn is_help_key(key: KeyEvent) -> bool {
    key.code == KeyCode::Char('?')
        && !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

#[cfg(test)]
mod tests;
