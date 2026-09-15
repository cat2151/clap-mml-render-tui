//! Chord Chart 画面の状態とキー処理。
//!
//! ここに置くのは状態そのものと、キーの振り分け。pane ごとの編集の中身は
//! 別モジュールへ出す（`section_edit` / `arrangement_edit`）。曲が変わったキーは
//! [`ChordChartAction::SongChanged`] を返し、**保存は呼び出し側（app の glue）が行う**
//! （この crate はログもファイル書き込みの失敗通知も持たないため）。

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::catalog::ChordProgressionSource;
use crate::{SectionId, Song};

use self::chord_cursor::ChordStep;
use self::line_input::LineInput;

mod arrangement_edit;
mod chord_cursor;
mod chord_ranges;
// 描画（`crate::ui`）が overlay の中身を読むので、screen の外から見える必要がある。
pub(crate) mod line_input;
mod preview;
mod section_edit;

pub use self::preview::{PreviewRequest, PreviewVoicingContext};

/// [`ChordChartScreen::handle_key_event`] の結果。
///
/// 画面の状態だけが変わったのか、保存すべき曲が変わったのかを呼び出し側へ伝える。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ChordChartAction {
    /// 曲は変わっていない（カーソル移動・pane 切替・ヘルプ開閉、または未定義のキー）。
    #[default]
    Continue,
    /// 曲が変わったので、呼び出し側は保存すること（デバウンス禁止＝そのまま即書き）。
    SongChanged,
    /// 試聴設定が変わったので、呼び出し側は session state を保存すること。
    ///
    /// 曲そのものは変わっていないため、`chord_chart.json` は保存しない。
    PreviewSettingChanged,
    /// `i`: 選択中の section の進行を host 側で編集する。
    ///
    /// この crate は編集 UI や degrees の解釈を持たず、安定した id だけを渡す。
    EditDegrees(SectionId),
    /// `q`: アプリを終了する。他の画面（grid sequencer / loop browser / notepad）と
    /// 同じ意味の `q` なので、この画面だけ別の意味を持たせない。
    Quit,
}

/// カーソルがどちらの pane にあるか。左 = 素材の定義、右 = 曲の並び。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Pane {
    /// 素材（section）の定義。
    #[default]
    Sections,
    /// 曲の並び（arrangement）。
    Arrangement,
}

/// `PgDn` / `PgUp` でカーソルが飛ぶ行数。
///
/// 画面の高さではなく固定値にしてある。この crate は描画領域の高さを知らない
/// （キー処理は `Frame` を見ない）ので、「1 画面ぶん」は原理的に決められない。
const PAGE_ROWS: usize = 10;

/// Chord Chart 画面 1 つぶんの状態。
#[derive(Clone, Debug)]
pub struct ChordChartScreen {
    pub song: Song,
    pub focus: Pane,
    /// 左 pane のカーソル行。範囲外になり得るので、読むときは
    /// [`ChordChartScreen::clamped_section_cursor`] を通すこと。
    pub section_cursor: usize,
    /// 右 pane のカーソル行。同上。
    pub arrangement_cursor: usize,
    pub help_open: bool,
    /// `n` / `b` の 1 行入力欄。開いている間は**全部のキーをここへ渡す**。
    /// 中身を読むのは描画だけなので `pub` にしない（開いているかどうかは
    /// [`ChordChartScreen::line_input_open`]）。
    pub(crate) line_input: Option<LineInput>,
    /// `dd` の 1 打目 `d` を押した状態。次のキーが `d` なら削除、それ以外なら捨てる。
    ///
    /// 画面には出さない（1 打目で状態が変わったことを見せる必要はない。vim も出さない）。
    /// crate 内で見えているのは、描画テストが `..ChordChartScreen::default()` で
    /// 組み立てられるようにするため（`line_input` と同じ理由）。
    pub(crate) pending_delete: bool,
    /// 保存ファイルが読めなかったので、**画面を最初に開いたときに `g` を 1 回だけ
    /// 自動で押す**。[`ChordChartScreen::enter`] が消費する。
    ///
    /// ここで抽選せず「開いたとき」まで遅らせるのは、カタログが遅延取得で、
    /// キャッシュがまだ無い初回は最大 20 秒待つため（アプリ全体の起動を止めない）。
    pub(crate) pending_initial_generate: bool,
    /// 立っている「鳴らせ」の要求。[`ChordChartScreen::take_preview`] が消費する。
    ///
    /// **音を出すのは app 側**（この crate は `MmlOverlaySender` を知らないし、
    /// degrees を解釈もしない）。ここに置くのは「何を鳴らすべきか」だけ。
    /// crate 内で見えているのは、描画テストが `..ChordChartScreen::default()` で
    /// 組み立てられるようにするため（`line_input` と同じ理由）。
    pub(crate) pending_preview: Option<PreviewRequest>,
    /// 直前に投げた preview がまだ鳴っている、と app 側が判断していること。
    ///
    /// **真偽を決めるのは app 側**（実際に送れたか、何秒の演奏だったかを知っているのは
    /// glue だけ）。ここはその答えを写しただけで、[`Self::toggle_preview`] が
    /// 「止める」と「鳴らす」のどちらを要求するかを決めるためだけに読む。
    /// 書き込みは [`ChordChartScreen::set_preview_sounding`] から。
    pub(crate) preview_sounding: bool,
    /// Chord Chart の section preview に Bass layer を重ねるか。
    ///
    /// 曲の内容ではなく session state。新規画面では ON で始め、復元値は
    /// [`ChordChartScreen::set_bass_enabled`] から書き戻す。
    pub(crate) bass_enabled: bool,
    /// 各 section の degrees の、chord 1 つぶんの範囲（**バイト位置**）の写し。
    /// chord が何個あるかも、この写しの長さから引く（数と範囲を別々に持たない）。
    ///
    /// **切るのは app 側**（この crate は degrees を解釈しない。ADR 0020）。
    /// 書き込みは [`ChordChartScreen::set_chord_ranges`] から。読むのは
    /// `chord_ranges` モジュールの関数だけで、他は素通しする。
    /// crate 内で見えているのは、描画テストが `..ChordChartScreen::default()` で
    /// 組み立てられるようにするため（`line_input` と同じ理由）。
    pub(crate) chord_ranges: BTreeMap<SectionId, Vec<Range<usize>>>,
    /// 行内の何番目の chord をカーソルが指しているか（**0 始まり**）。
    ///
    /// **曲のデータではない**ので保存しない。行を移ると先頭へ戻る。範囲外のまま
    /// 残りうる（degrees を打ち替えて chord が減ったとき）ので、読むときは
    /// [`ChordChartScreen::chord_cursor`] を通すこと。
    /// crate 内で見えているのは、描画テストが `..ChordChartScreen::default()` で
    /// 組み立てられるようにするため（`line_input` と同じ理由）。
    pub(crate) chord_cursor: usize,
    /// 直前の操作が何もできなかった理由。画面下段に出す。
    pub error: Option<String>,
    /// コード進行カタログの供給元。注入されるまでは `None`（＝抽選は
    /// 「コード進行データがありません」になる）。**引くのは抽選するときだけ**
    /// （`g` / `r` と、保存ファイルが読めなかったときの [`ChordChartScreen::enter`]）。
    ///
    /// crate の外から差し替えるのは [`ChordChartScreen::set_chord_progression_source`]
    /// だけ（他のフィールドと違い `pub` にしない）。crate 内で見えているのは、
    /// 描画テストが `..ChordChartScreen::default()` で組み立てられるようにするため。
    pub(crate) chord_progression_source: Option<ChordProgressionSource>,
}

impl Default for ChordChartScreen {
    fn default() -> Self {
        Self::new(Song::empty())
    }
}

impl ChordChartScreen {
    pub fn new(song: Song) -> Self {
        Self {
            song,
            focus: Pane::Sections,
            section_cursor: 0,
            arrangement_cursor: 0,
            help_open: false,
            line_input: None,
            pending_delete: false,
            pending_initial_generate: false,
            pending_preview: None,
            preview_sounding: false,
            bass_enabled: true,
            chord_ranges: BTreeMap::new(),
            chord_cursor: 0,
            error: None,
            chord_progression_source: None,
        }
    }

    /// 保存ファイルから復元する。`None`（読めなかった）なら空から始め、
    /// **画面を最初に開いたときに 1 回だけ自動で抽選する**（[`Self::enter`]）。
    ///
    /// 読めた曲が section 0 個でも抽選しない。全部消してから再起動したときに、
    /// 消したはずの section が復活しないため。
    pub fn restored(saved: Option<Song>) -> Self {
        let pending_initial_generate = saved.is_none();
        let mut screen = Self::new(saved.unwrap_or_else(Song::empty));
        screen.pending_initial_generate = pending_initial_generate;
        screen
    }

    /// 画面へ入るたびに呼ぶ。保存ファイルが読めなかったときだけ、`g` と同じ抽選を
    /// **1 回だけ**行って section 1 つと arrangement 1 行を作る。
    ///
    /// 曲を作れたら [`ChordChartAction::SongChanged`] を返す。呼び出し側は保存すること
    /// （保存しないと、次の起動でまた別の進行が抽選される）。カタログが無くて抽選
    /// できなかったときは空のまま `Continue` を返し、理由を下段に出す。
    /// 失敗しても「1 回だけ」は使い切る（開き直すたびにカタログ取得を待たされない）。
    /// **画面を開いた直後の preview 要求もここで立てる**（抽選が走った場合はその
    /// section が対象になる。抽選は要求を立てるより先に済ませること）。
    pub fn enter(&mut self) -> ChordChartAction {
        let action = if std::mem::take(&mut self.pending_initial_generate) {
            self.generate_initial_section()
        } else {
            ChordChartAction::Continue
        };
        self.request_preview_on_enter();
        action
    }

    /// コード進行カタログの供給元を注入する。
    ///
    /// 渡すのは**遅延評価のクロージャ**にすること。キャッシュがまだ無い初回は
    /// 中でネットワーク取得の完了を待つので、起動時や画面を開いた時点で呼ばれると
    /// そのぶん止まる。この画面が呼ぶのは `g` / `r` を押した瞬間と、保存ファイルが
    /// 読めなかったときの [`ChordChartScreen::enter`] だけ。
    pub fn set_chord_progression_source(
        &mut self,
        source: Arc<dyn Fn() -> Vec<String> + Send + Sync>,
    ) {
        self.chord_progression_source = Some(ChordProgressionSource::new(source));
    }

    fn chord_progression_source(&self) -> Option<&ChordProgressionSource> {
        self.chord_progression_source.as_ref()
    }

    /// 実際に描くべき左 pane のカーソル行。行が無いときは 0。
    ///
    /// 編集で行が減ると保持した index が溢れる。**丸めるのは描画側の
    /// 責任にせず、ここ 1 か所に閉じる。**
    pub fn clamped_section_cursor(&self) -> usize {
        clamp_cursor(self.section_cursor, self.song.sections.len())
    }

    /// 実際に描くべき右 pane のカーソル行。行が無いときは 0。
    pub fn clamped_arrangement_cursor(&self) -> usize {
        clamp_cursor(self.arrangement_cursor, self.song.arrangement.len())
    }

    /// 左 pane のカーソルが指している section。
    pub fn selected_section(&self) -> Option<&crate::Section> {
        self.song.sections.get(self.clamped_section_cursor())
    }

    /// キー 1 つを処理する。
    ///
    /// `Ctrl+G`（画面切替）や `Ctrl+P`（MML オーバーレイ）は共有ランタイムが先に食うので、
    /// ここへは来ない。念のため CONTROL 付きは何もせず返す
    /// （将来ランタイム側が取りこぼしても、この画面が勝手に反応しないため）。
    /// **ALT は `Alt+↑` `Alt+↓` だけ使う**ので、ガードより手前で拾う。
    pub fn handle_key_event(&mut self, key: KeyEvent) -> ChordChartAction {
        // 1 行入力欄が開いている間は、CONTROL 付きも含めて全部のキーを入力欄へ渡す。
        // `Ctrl+W`(単語削除) / `Ctrl+U`(undo) は 1 行入力欄として当然効くべきキーなので、
        // **CONTROL / ALT を弾くガードより手前**でなければならない。
        if self.line_input.is_some() {
            return self.handle_line_input_key(key);
        }
        // ヘルプを開いている間は閉じるキーだけを見る。裏の画面が動くと、
        // 閉じたときに何が起きたのか分からなくなる。
        if self.help_open {
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                && matches!(key.code, KeyCode::Char('?') | KeyCode::Esc)
            {
                self.help_open = false;
            }
            return ChordChartAction::Continue;
        }
        // `dd` の 2 打目。`d` 以外だった保留はここで捨て、そのキーは通常どおり処理する
        // （捨てたキーを一緒に握り潰すと、押したのに何も起きない 1 回ができる）。
        let pending_delete = std::mem::take(&mut self.pending_delete);
        if pending_delete && is_plain(key) && key.code == KeyCode::Char('d') {
            self.error = None;
            return self.delete_under_cursor();
        }
        // `Alt+↑` `Alt+↓` は ALT 付きで届く。CONTROL / ALT ガードより手前で拾わないと、
        // ガードに弾かれて無反応になる。
        if key.modifiers.contains(KeyModifiers::ALT)
            && !key.modifiers.contains(KeyModifiers::CONTROL)
        {
            self.error = None;
            return match key.code {
                KeyCode::Up => self.move_row(MoveDirection::Up),
                KeyCode::Down => self.move_row(MoveDirection::Down),
                _ => ChordChartAction::Continue,
            };
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return ChordChartAction::Continue;
        }
        // `error` は「直前の操作ができなかった理由」なので、次の操作を始めた時点で古い。
        // 新しい理由は、このあとの各キーの処理が改めて立てる。
        self.error = None;
        match key.code {
            KeyCode::Char('?') => self.help_open = true,
            KeyCode::Char('q') => return ChordChartAction::Quit,
            // pane 移動は `Tab` のトグル（2 pane しか無い）。`h` / `l` は行内の
            // chord 移動へ譲った。`Shift+Tab`（`BackTab`）は割り当てない。
            KeyCode::Tab => self.toggle_pane(),
            // 行内の chord 移動。行の端では隣の行へ繰り上がる。
            KeyCode::Char('h') | KeyCode::Left => self.move_chord_cursor(ChordStep::Previous),
            KeyCode::Char('l') | KeyCode::Right => self.move_chord_cursor(ChordStep::Next),
            KeyCode::Char('j') | KeyCode::Down => self.move_cursor(CursorStep::Next(1)),
            KeyCode::Char('k') | KeyCode::Up => self.move_cursor(CursorStep::Previous(1)),
            KeyCode::PageDown => self.move_cursor(CursorStep::Next(PAGE_ROWS)),
            KeyCode::PageUp => self.move_cursor(CursorStep::Previous(PAGE_ROWS)),
            // `dd` の 1 打目。ここでは何も消さない（`d` 単独では消えない）。
            KeyCode::Char('d') => self.pending_delete = true,
            // `b` は pane に関係なく曲そのもの（prefix）を書き換えるので共通キー。
            KeyCode::Char('b') => return self.open_prefix_line_input(),
            // `B`: Bass preview を切り替え、新しい状態で現在 section を頭から鳴らす。
            KeyCode::Char('B') => {
                self.toggle_bass_enabled();
                return ChordChartAction::PreviewSettingChanged;
            }
            // `Shift+P` / `Space`: preview のトグル。両 pane 共通キー。
            // `P` は SHIFT 付きで届くが、`is_plain` が SHIFT を許すのでここまで来る。
            KeyCode::Char('P') | KeyCode::Char(' ') => self.toggle_preview(),
            // ここから下は pane ごとに意味が変わるキー。
            code => return self.handle_pane_key(code),
        }
        ChordChartAction::Continue
    }

    /// `dd`: フォーカスしている pane の行を消す。左 = section、右 = arrangement 行。
    fn delete_under_cursor(&mut self) -> ChordChartAction {
        match self.focus {
            Pane::Sections => self.delete_selected_section(),
            Pane::Arrangement => self.delete_arrangement_entry(),
        }
    }

    /// `Alt+↑` `Alt+↓`: フォーカスしている pane のカーソル行を 1 つ動かす。
    ///
    /// 行が動いたら chord カーソルは先頭へ戻す（他の行移動キーと同じ）。**preview は
    /// 立てない**（編集キーなので、中身が動いただけでは鳴らし直さない）。
    fn move_row(&mut self, direction: MoveDirection) -> ChordChartAction {
        let action = match self.focus {
            Pane::Sections => self.move_section(direction),
            Pane::Arrangement => self.move_arrangement_entry(direction),
        };
        if action == ChordChartAction::SongChanged {
            self.reset_chord_cursor();
        }
        action
    }

    /// pane によって意味が変わるキー。フォーカスしていない pane のキーは何もしない。
    fn handle_pane_key(&mut self, code: KeyCode) -> ChordChartAction {
        match self.focus {
            Pane::Sections => self.handle_sections_key(code),
            Pane::Arrangement => self.handle_arrangement_key(code),
        }
    }

    /// Sections pane（左）でだけ効くキー。
    fn handle_sections_key(&mut self, code: KeyCode) -> ChordChartAction {
        match code {
            KeyCode::Char('g') => self.add_section_from_catalog(),
            KeyCode::Char('r') => self.reroll_selected_section(),
            KeyCode::Char('i') => self
                .selected_section()
                .map_or(ChordChartAction::Continue, |section| {
                    ChordChartAction::EditDegrees(section.id)
                }),
            KeyCode::Char('n') => self.open_name_line_input(),
            _ => ChordChartAction::Continue,
        }
    }

    /// フォーカスしている pane のカーソルを動かす。`j` `k` は 1 行、`PgDn` `PgUp` は
    /// [`PAGE_ROWS`] 行。端で止まる（行き過ぎても反対側へは回らない）。
    ///
    /// 動かす前に必ず丸めた値から数える。丸めずに足し引きすると、行が減ったあとの
    /// 溢れた index（例: 行 2 つに対し 99）から `k` を押しても画面上のカーソルが
    /// 何十回も動かないように見える。
    ///
    /// カーソルが実際に動いたときだけ preview 要求を立てる（端で止まったときは
    /// 音を鳴らし直さない）。鳴らすのは**行全体**（chord カーソルは先頭へ戻る）。
    fn move_cursor(&mut self, step: CursorStep) {
        let before = self.cursor_position();
        let (current, len) = match self.focus {
            Pane::Sections => (self.clamped_section_cursor(), self.song.sections.len()),
            Pane::Arrangement => (
                self.clamped_arrangement_cursor(),
                self.song.arrangement.len(),
            ),
        };
        let next = match step {
            CursorStep::Next(rows) => (current + rows).min(len.saturating_sub(1)),
            CursorStep::Previous(rows) => current.saturating_sub(rows),
        };
        match self.focus {
            Pane::Sections => self.section_cursor = next,
            Pane::Arrangement => self.arrangement_cursor = next,
        }
        // 行が変わったら chord カーソルは先頭へ。端で止まったときに戻すと、
        // 「押しても行は動かないのに聴いている chord だけ変わる」1 回ができる。
        if next != current {
            self.reset_chord_cursor();
        }
        self.request_preview_if_moved(before);
    }
}

/// カーソルを何行動かすか。`j` `k` は 1、`PgDn` `PgUp` は [`PAGE_ROWS`]。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CursorStep {
    Next(usize),
    Previous(usize),
}

/// `Alt+↑` `Alt+↓` で行を動かす向き。左右どちらの pane でも同じ意味。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MoveDirection {
    Up,
    Down,
}

/// 修飾なし（`SHIFT` だけは付いていてよい）のキーか。
///
/// `SHIFT` を許すのは、記号や大文字が SHIFT 付きで届くため。
fn is_plain(key: KeyEvent) -> bool {
    !key.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

fn clamp_cursor(cursor: usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    cursor.min(len - 1)
}

#[cfg(test)]
mod tests;
