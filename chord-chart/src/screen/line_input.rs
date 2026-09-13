//! 1 行入力 overlay。`n` で名前(name)、`b` で曲頭の指定(prefix)。
//!
//! overlay は 1 個で、**対象だけを切り替える**（同時に 2 つ開くことは無い）。
//! 対象が section とは限らない（`b` は曲そのものを書き換える）ので、モジュール名は
//! pane ではなく「1 行入力欄」という役割で付けてある。
//! 入力欄の実体は `ratatui-textarea` の [`TextArea`]。自前で `String` へ push/pop
//! しないのは、Shift+カーソルの範囲選択・`Ctrl+W` の単語削除・`Ctrl+U`/`Ctrl+R` の
//! undo/redo・`Home`/`End` といった「1 行入力なら当然効くべき」keybind を
//! 落とさないため（プロジェクト共通の作法）。
//!
//! **エラーは overlay 自身が持つ**。画面下段の [`ChordChartScreen::error`] は
//! 「直前の操作ができなかった理由」で次のキーを押した時点で消える設計なので、
//! 「打ち間違いを直すまで出しっぱなし」には使えない。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::TextArea;

use super::{ChordChartAction, ChordChartScreen};

/// 名前を空のまま確定しようとしたときの理由。
///
/// 空を通すと左 pane の行が名前無しで並び、`1`..`9` で何を挿しているのか読めなくなる。
pub(crate) const EMPTY_NAME_MESSAGE: &str = "名前を入力してください";

/// 入力欄が何を書き換えるのか。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LineInputTarget {
    /// `n`: 画面に出す短い名前。空でなければ何でもよい。
    Name,
    /// `b`: 曲頭に置く chord2mml の指定（`Key=A BPM120`）。
    ///
    /// **一切検証しない**（空文字も通す）。書式は chord2mml-rs のものをそのまま使うので、
    /// ここでパースし直すのはムダ。読めない文字列を弾く責任は演奏側にある（別スコープ）。
    Prefix,
}

impl LineInputTarget {
    /// overlay の枠のタイトル。
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Name => " 名前(name) ",
            Self::Prefix => " Key / BPM ",
        }
    }

    /// 空のときに薄く出す案内。何を書く欄なのかを字で示す。
    ///
    pub(crate) fn placeholder(self) -> &'static str {
        match self {
            Self::Name => "Sabi",
            Self::Prefix => crate::song::DEFAULT_PREFIX,
        }
    }
}

/// 開いている 1 行入力欄 1 つぶん。
#[derive(Clone, Debug)]
pub(crate) struct LineInput {
    target: LineInputTarget,
    textarea: TextArea<'static>,
    error: Option<String>,
}

impl LineInput {
    fn new(target: LineInputTarget, text: &str) -> Self {
        Self {
            target,
            textarea: cmrt_tui_core::text_input::new_single_line_textarea(text),
            error: None,
        }
    }

    pub(crate) fn target(&self) -> LineInputTarget {
        self.target
    }

    pub(crate) fn textarea(&self) -> &TextArea<'static> {
        &self.textarea
    }

    pub(crate) fn value(&self) -> String {
        cmrt_tui_core::text_input::textarea_value(&self.textarea)
    }

    /// 確定できなかった理由。直すまで出しっぱなしにする。
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

impl ChordChartScreen {
    /// 1 行入力欄が開いているか。
    ///
    /// app 側が「端末カーソルを入力欄に置くか」（`uses_textarea_cursor`）と
    /// 「`Ctrl+G` で画面切替メニューを開いてよいか」を決めるのに使う。
    pub fn line_input_open(&self) -> bool {
        self.line_input.is_some()
    }

    pub(crate) fn line_input(&self) -> Option<&LineInput> {
        self.line_input.as_ref()
    }

    /// `n`: カーソル section の名前を初期値にして入力欄を開く。
    ///
    /// section が 1 つも無いときは開かない（書き込む先が無い）。`r` / `dd` が
    /// 同じ場面で黙って `Continue` を返すのに合わせてある。
    pub(super) fn open_name_line_input(&mut self) -> ChordChartAction {
        let Some(section) = self.selected_section() else {
            return ChordChartAction::Continue;
        };
        self.line_input = Some(LineInput::new(LineInputTarget::Name, &section.name));
        ChordChartAction::Continue
    }

    /// `b`: 曲頭の指定を今の値のまま入力欄へ入れて開く。
    ///
    /// section が 0 個でも開く（書き込む先は曲そのもので、カーソル行ではない）。
    pub(super) fn open_prefix_line_input(&mut self) -> ChordChartAction {
        let initial = self.song.prefix.clone();
        self.line_input = Some(LineInput::new(LineInputTarget::Prefix, &initial));
        ChordChartAction::Continue
    }

    /// 入力欄が開いている間の全キー。`Esc` で破棄、確定キーで確定、残りは入力欄へ。
    pub(super) fn handle_line_input_key(&mut self, key: KeyEvent) -> ChordChartAction {
        let Some(mut input) = self.line_input.take() else {
            return ChordChartAction::Continue;
        };
        if key.code == KeyCode::Esc {
            // 破棄。取り出したまま戻さないので閉じる。
            return ChordChartAction::Continue;
        }
        if is_commit_key(key) {
            match self.commit_line_input(&input) {
                Ok(action) => return action,
                Err(reason) => {
                    // 閉じずに理由を出したまま入力を続けさせる。閉じてしまうと、
                    // 打った文字列ごと消えて最初から打ち直しになる。
                    input.error = Some(reason);
                    self.line_input = Some(input);
                    return ChordChartAction::Continue;
                }
            }
        }
        if cmrt_tui_core::text_input::apply_key_event_to_textarea(&mut input.textarea, key) {
            // 文字が変わった＝直している最中なので、古い理由は消す。
            input.error = None;
        }
        self.line_input = Some(input);
        ChordChartAction::Continue
    }

    /// 確定。`Err` は「閉じずに出す理由」、`Ok` は閉じたうえでの結果。
    fn commit_line_input(&mut self, input: &LineInput) -> Result<ChordChartAction, String> {
        let value = input.value().trim().to_string();
        match input.target() {
            LineInputTarget::Prefix => {
                if self.song.prefix == value {
                    return Ok(ChordChartAction::Continue);
                }
                // **検証しない**。打った文字列をそのまま持つ（空文字も含めて何でも受ける）。
                self.song.prefix = value;
            }
            LineInputTarget::Name => {
                let Some(index) = self.selected_section_index() else {
                    return Ok(ChordChartAction::Continue);
                };
                if value.is_empty() {
                    return Err(EMPTY_NAME_MESSAGE.to_string());
                }
                if self.song.sections[index].name == value {
                    return Ok(ChordChartAction::Continue);
                }
                self.song.sections[index].name = value;
            }
        }
        Ok(ChordChartAction::SongChanged)
    }
}

/// 1 行入力欄の確定キー。
///
/// 端末では `Ctrl+M` が `Enter` と同じバイトで届くことがあるが、crossterm は
/// `Ctrl+M` として渡してくることもあるので両方拾う（グローバルの 1 行入力欄の作法。
/// 先例: `loop-browser/src/filter.rs` の `is_commit_key`）。
fn is_commit_key(key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Enter => true,
        KeyCode::Char('m') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
