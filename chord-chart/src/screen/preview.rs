//! 「いま何を鳴らすべきか」の要求（preview）。**この crate は音を出さない。**
//!
//! 鳴らすのは app 側の glue（`MmlOverlaySender` を持っているのは app）。ここは
//! 「カーソルが動いた」「画面へ入った」を音の要求へ翻訳して置いておくだけで、
//! degrees も prefix も**解釈しない**（ADR 0020）。
//!
//! 要求が立つのはカーソルの位置（pane と行）が**実際に変わったとき**だけ。
//! 端で止まって行が変わらなかった `j`、いる pane をもう一度指した `h` では立たない
//! （押すたびに鳴らし直すと、端で連打したときに同じ音が何度も鳴る）。

use super::{ChordChartScreen, Pane};
use crate::Section;

/// 1 回ぶんの「鳴らせ」。中身は section の degrees と、ログに出す名前だけ。
///
/// **`Option<PreviewRequest>` の `None` とは別物**。`None` は「何も起きていない」で、
/// [`PreviewRequest::is_silent`] な要求は「前の音を止めろ」。行が無い / 参照が
/// 壊れている / degrees が空のときは後者になる（無反応にすると、空の行へ降りても
/// 前の section が鳴りっぱなしになる）。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PreviewRequest {
    /// 鳴らす section の名前。ログと、鳴らせなかったときの理由に使う。行が無ければ空。
    pub name: String,
    /// 鳴らす degrees。**解釈しない**（読めるかどうかを決めるのは演奏側）。
    pub degrees: String,
}

impl PreviewRequest {
    /// 「止めるだけ」の要求。
    pub fn silent() -> Self {
        Self::default()
    }

    /// 鳴らすものが無い＝前の音を止めるだけの要求か。
    ///
    /// 空白だけの degrees も無音に含める。`" "` を鳴らそうとしても音にならないのに、
    /// 前の音だけが止まらない状態になる。
    pub fn is_silent(&self) -> bool {
        self.degrees.trim().is_empty()
    }
}

impl ChordChartScreen {
    /// 立っている preview 要求を取り出す（取り出したら消える）。
    ///
    /// 呼ぶのは app の glue。1 回のキー処理で 2 回鳴らないよう、必ず消費すること。
    pub fn take_preview(&mut self) -> Option<PreviewRequest> {
        self.pending_preview.take()
    }

    /// 画面を開いた直後の 1 回。行が無ければ無音要求になる。
    pub(super) fn request_preview_on_enter(&mut self) {
        self.request_preview();
    }

    /// **鳴っているかどうかの答えを app 側から受け取る。**
    ///
    /// この crate は音を出さないので、鳴っているかを自分では知り得ない。判定材料を
    /// 持っているのは glue だけ（sender があるか、何秒の演奏を投げたか）。
    /// glue は preview を投げるたび・キーを渡す直前にこれを書き戻すこと。
    ///
    /// **`MmlOverlaySenderStatus::sounding()` は使えない**（2026-09-09 実測。
    /// あれは打鍵の生 MIDI 専用の欄で、行の演奏では空のまま。
    /// `mml-overlay/src/sender/tests.rs` の
    /// `a_line_performance_leaves_the_sounding_status_empty` が固定している）。
    pub fn set_preview_sounding(&mut self, sounding: bool) {
        self.preview_sounding = sounding;
    }

    /// 直前に投げた preview が鳴っている、と app 側が言っているか。
    pub fn preview_sounding(&self) -> bool {
        self.preview_sounding
    }

    /// `Shift+P` / `Space`: 鳴っていたら止め、止まっていたらカーソル行を鳴らす。
    ///
    /// 2 キーは同じ動作（3.3）。止めるほうも「無音を鳴らせ」という要求として立てる
    /// ので、glue から見ると空の行へカーソルを移したときと同じ 1 本の経路になる
    /// （`LineProgram::silent()` は sender の `Stop` と同じく音源を止める。
    /// `mml-overlay/src/sender/tests.rs` の
    /// `an_empty_line_stops_the_running_timeline` が固定している）。
    pub(super) fn toggle_preview(&mut self) {
        if self.preview_sounding {
            self.pending_preview = Some(PreviewRequest::silent());
        } else {
            self.request_preview();
        }
    }

    /// `h` / `l`: pane を指す。移った先のカーソル行を鳴らす。
    pub(super) fn focus_pane(&mut self, pane: Pane) {
        let before = self.cursor_position();
        self.focus = pane;
        self.request_preview_if_moved(before);
    }

    /// カーソル位置が [`Self::cursor_position`] と違っていたら要求を立てる。
    pub(super) fn request_preview_if_moved(&mut self, before: (Pane, usize)) {
        if self.cursor_position() != before {
            self.request_preview();
        }
    }

    /// いまカーソルがどこを指しているか。preview を立てるかどうかの判定は
    /// **この値が変わったかどうか**だけで決める（pane と行を 1 つの値にしておくと、
    /// `j` と `h` で別々の判定を書かずに済む）。
    ///
    /// 丸めた値で持つ。丸める前の溢れた index を使うと、行が減ったあとの `k` が
    /// 「画面上のカーソルは動かないのに要求だけ立つ」1 回になる。
    pub(super) fn cursor_position(&self) -> (Pane, usize) {
        match self.focus {
            Pane::Sections => (Pane::Sections, self.clamped_section_cursor()),
            Pane::Arrangement => (Pane::Arrangement, self.clamped_arrangement_cursor()),
        }
    }

    /// いまカーソルが指しているものを鳴らす要求を立てる。
    fn request_preview(&mut self) {
        let request = match self.preview_target() {
            Some(section) => PreviewRequest {
                name: section.name.clone(),
                degrees: section.degrees.clone(),
            },
            None => PreviewRequest::silent(),
        };
        self.pending_preview = Some(request);
    }

    /// カーソルが指している section。右 pane では**参照先**の section
    /// （並びの行そのものは進行を持たない）。参照が壊れていれば `None`。
    fn preview_target(&self) -> Option<&Section> {
        match self.focus {
            Pane::Sections => self.selected_section(),
            Pane::Arrangement => {
                let id = *self
                    .song
                    .arrangement
                    .get(self.clamped_arrangement_cursor())?;
                self.song.section(id)
            }
        }
    }
}

#[cfg(test)]
mod tests;
