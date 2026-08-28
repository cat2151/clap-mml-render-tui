//! 「MML のつもりで打った文字列がコード表記だった」ときの検出と、chord 行への移送。
//!
//! **これは言語を決める判定ではない。** どの言語かは行の位置で決まる（chord 行は
//! コード進行、演奏 track は MML）。ここでやるのは間違いの検出だけで、
//! MML パーサが未知の文字を黙って捨てる（`I` はノート 0 個、`Cm7` は単音の C）ため、
//! chord をパースしてみる以外に気づく手段が無い。
//!
//! 出し方は 2 段構え。
//!
//! 1. 打鍵のたびに静かなヒントを 1 行出す。**ダイアログは出さない**
//!    （打つたびに手が止まると、意図してコードを書き写している人の邪魔になる）。
//! 2. 確定（`Enter` / `Esc`）のときだけダイアログを出す。**最後の砦。**
//!    選択肢は「chord 行へ移す」か「このまま MML として確定する」で、
//!    **破棄は無い**（打った文字列が黙って消える出口を作らない）。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// 入力欄の下に出す 1 行。
///
/// 資料の原案は「`C` で chord 行へ」だったが、**overlay を開いている間の `C` は
/// 入力欄への 1 文字**（`C` が chord 行ジャンプになるのは DAW の normal mode）。
/// 実際に移せるのは確定キーなので、そう書く。
pub(crate) const CHORD_HINT: &str = "chord として解釈できます。Enter で chord 行へ移せます";

/// 確定ダイアログの選択肢。並びがそのまま表示順とカーソル位置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChordTransferChoice {
    /// chord 行の同じ小節へ移す。編集中のセルへは書かない。
    Transfer,
    /// 打った文字列をそのまま MML として確定する。
    KeepAsMml,
}

impl ChordTransferChoice {
    pub(crate) const ALL: [Self; 2] = [Self::Transfer, Self::KeepAsMml];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Transfer => "chord 行へ移す",
            Self::KeepAsMml => "このまま MML として確定",
        }
    }

    pub(crate) fn detail(self) -> &'static str {
        match self {
            Self::Transfer => "同じ小節の chord 行へ書く",
            Self::KeepAsMml => "無音か単音になります",
        }
    }
}

/// ダイアログが呼び出し側へ求める処理。
pub(crate) enum ChordTransferAction {
    /// カーソルが動いただけ。まだ閉じない。
    Continue,
    /// 選択を確定した。
    Confirm(ChordTransferChoice),
    /// 確定そのものを取り消して入力欄へ戻る。**何も書かない。**
    Cancel,
}

/// 確定の直前に立つダイアログ。
///
/// 確定しようとした 1 行と、その確定が overlay を閉じる確定だったか（`Esc`）を
/// 抱えておく。「このまま MML として確定」を選んだときに、ダイアログが無かった場合と
/// 1 ビットも変わらない [`crate::MmlOverlayAction::Commit`] を返すため。
pub(crate) struct ChordTransferConfirm {
    line: String,
    close: bool,
    cursor: usize,
}

impl ChordTransferConfirm {
    pub(crate) fn open(line: String, close: bool) -> Self {
        Self {
            line,
            close,
            // 既定は移送。ここを「MML のまま」にすると、ダイアログを読まずに
            // Enter を続けて押したときに発端のバグ（無音のセル）がそのまま通る。
            cursor: 0,
        }
    }

    pub(crate) fn line(&self) -> &str {
        &self.line
    }

    pub(crate) fn close(&self) -> bool {
        self.close
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> ChordTransferAction {
        if is_cancel_key(key) {
            return ChordTransferAction::Cancel;
        }
        match key.code {
            KeyCode::Enter => ChordTransferAction::Confirm(self.selected()),
            KeyCode::Up | KeyCode::Left => self.move_cursor(-1),
            KeyCode::Down | KeyCode::Right | KeyCode::Tab => self.move_cursor(1),
            _ => ChordTransferAction::Continue,
        }
    }

    fn selected(&self) -> ChordTransferChoice {
        ChordTransferChoice::ALL[self.cursor.min(ChordTransferChoice::ALL.len() - 1)]
    }

    fn move_cursor(&mut self, delta: isize) -> ChordTransferAction {
        let last = ChordTransferChoice::ALL.len() - 1;
        self.cursor = self.cursor.saturating_add_signed(delta).min(last);
        ChordTransferAction::Continue
    }
}

/// このキーは確定を取り消して入力欄へ戻る。
///
/// `Esc` で開いたダイアログを `Esc` で閉じても、閉じずに入力欄へ戻る
/// （`Esc` の意味が「確定して閉じる」なので、取り消せば閉じない）。
fn is_cancel_key(key: KeyEvent) -> bool {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return false;
    }
    key.code == KeyCode::Esc
}
