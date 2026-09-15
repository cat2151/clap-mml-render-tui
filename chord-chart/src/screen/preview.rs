//! 「いま何を鳴らすべきか」の要求（preview）。この crate は音を出さない。
//!
//! 鳴らすのは app 側の glue（`MmlOverlaySender` を持っているのは app）。ここは「カーソルが
//! 動いた」「画面へ入った」を音の要求へ翻訳して置くだけで、degrees も prefix も解釈しない
//! （`docs/adr/0020`）。要求が立つのはカーソルの位置が実際に変わったときだけ。端で止まった
//! `j` や、いる pane をもう一度指した `h` では立たない（端で連打すると同じ音が何度も鳴る）。

use super::{ChordChartScreen, Pane};
use crate::Section;

/// auto voicing を決めるときに同じ並びとして扱う section 群。
///
/// Sections pane では選択 section だけ、Arrangement pane では曲順全体を入れる。
/// この crate は文字列を解釈せず、どの進行同士が隣接するかだけを app 側へ渡す。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PreviewVoicingContext {
    pub progressions: Vec<String>,
    /// [`Self::progressions`] のうち、実際に preview する section の位置。
    pub selected: usize,
}

/// 1 回ぶんの「鳴らせ」。中身は section の degrees と、ログに出す名前だけ。
///
/// `Option<PreviewRequest>` の `None` は「何も起きていない」、[`PreviewRequest::is_silent`] な
/// 要求は「前の音を止めろ」。行が無い / 参照が壊れている / degrees が空のときは後者になる
/// （無反応にすると、空の行へ降りても前の section が鳴りっぱなしになる）。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PreviewRequest {
    /// 鳴らす section の名前。ログと、鳴らせなかったときの理由に使う。行が無ければ空。
    pub name: String,
    /// 鳴らす degrees。解釈しない（読めるかどうかを決めるのは演奏側）。
    pub degrees: String,
    /// degrees の行内の何番目（0 始まり。画面の chord カーソルと同じ値）の chord を鳴らすか。
    /// `None` なら行全体。どこからどこまでかを決めるのは app 側の glue で、範囲外の番号や
    /// 読めない degrees は glue が行全体へ倒す（`docs/adr/0020`）。
    pub chord_index: Option<usize>,
    /// section 単独または Arrangement 全体の auto voicing 文脈。
    pub voicing_context: PreviewVoicingContext,
}

impl PreviewRequest {
    /// 1 section の中だけで auto voicing する preview 要求。
    pub fn section(
        name: impl Into<String>,
        degrees: impl Into<String>,
        chord_index: Option<usize>,
    ) -> Self {
        let degrees = degrees.into();
        Self {
            name: name.into(),
            voicing_context: PreviewVoicingContext {
                progressions: vec![degrees.clone()],
                selected: 0,
            },
            degrees,
            chord_index,
        }
    }

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
    /// Chord Chart の section preview に Bass layer を重ねる設定か。
    pub fn bass_enabled(&self) -> bool {
        self.bass_enabled
    }

    /// Session state から Bass preview 設定を復元する。
    ///
    /// 復元時には音を鳴らさないため、preview request は立てない。
    pub fn set_bass_enabled(&mut self, enabled: bool) {
        self.bass_enabled = enabled;
    }

    /// Bass preview を切り替え、新しい状態で現在 section 全体を頭から要求する。
    pub(super) fn toggle_bass_enabled(&mut self) {
        self.bass_enabled = !self.bass_enabled;
        self.request_preview();
    }

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

    /// 鳴っているかどうかの答えを app 側から受け取る。
    ///
    /// この crate は音を出さないので自分では知り得ない。判定材料を持つ glue が、preview を
    /// 投げるたび・キーを渡す直前に書き戻す。`MmlOverlaySenderStatus::sounding()` は打鍵の
    /// 生 MIDI 専用で行の演奏では空のまま（`mml-overlay/src/sender/tests.rs` の
    /// `a_line_performance_leaves_the_sounding_status_empty`）なので使えない。
    pub fn set_preview_sounding(&mut self, sounding: bool) {
        self.preview_sounding = sounding;
    }

    /// 直前に投げた preview が鳴っている、と app 側が言っているか。
    pub fn preview_sounding(&self) -> bool {
        self.preview_sounding
    }

    /// `Shift+P` / `Space`: 鳴っていたら止め、止まっていたらカーソル行を鳴らす。
    ///
    /// 止めるほうも「無音を鳴らせ」の要求として立てるので、glue から見ると空の行へカーソルを
    /// 移したときと同じ 1 本の経路になる（`LineProgram::silent()` は sender の `Stop` と同じく
    /// 音源を止める。`mml-overlay/src/sender/tests.rs` の `an_empty_line_stops_the_running_timeline`）。
    pub(super) fn toggle_preview(&mut self) {
        if self.preview_sounding {
            self.pending_preview = Some(PreviewRequest::silent());
        } else {
            self.request_preview();
        }
    }

    /// `Tab`: pane を指す。移った先のカーソル行を**行全体で**鳴らし、
    /// chord カーソルは先頭へ戻す（行が変わるので、番号を持ち越さない）。
    pub(super) fn focus_pane(&mut self, pane: Pane) {
        let before = self.cursor_position();
        self.focus = pane;
        self.reset_chord_cursor();
        self.request_preview_if_moved(before);
    }

    /// カーソル位置が [`Self::cursor_position`] と違っていたら要求を立てる。
    pub(super) fn request_preview_if_moved(&mut self, before: (Pane, usize, usize)) {
        if self.cursor_position() != before {
            self.request_preview();
        }
    }

    /// いまカーソルがどこを指しているか（pane・行・行内の chord 番号）。preview を立てるかは
    /// この値が変わったかどうかだけで決める（`j` と `Tab` と `l` で別々の判定を書かずに済む）。
    ///
    /// 丸めた値で持つ。丸める前の溢れた index を使うと、行が減ったあとの `k` が
    /// 「画面上のカーソルは動かないのに要求だけ立つ」1 回になる。
    pub(super) fn cursor_position(&self) -> (Pane, usize, usize) {
        let (pane, row) = match self.focus {
            Pane::Sections => (Pane::Sections, self.clamped_section_cursor()),
            Pane::Arrangement => (Pane::Arrangement, self.clamped_arrangement_cursor()),
        };
        (pane, row, self.chord_cursor())
    }

    /// いまカーソルが指している行を行全体で鳴らす要求を立てる（行移動・画面へ入った直後・トグル）。
    /// chord 1 つに絞るのは [`Self::request_chord_preview`] だけ。
    fn request_preview(&mut self) {
        self.request_preview_of(None);
    }

    /// いまカーソルが指している chord 1 つを鳴らす要求を立てる（`h` `l` `←` `→`）。
    /// 番号を切り出すのは app 側の glue なので、この crate は丸めた番号をそのまま渡すだけ。
    pub(super) fn request_chord_preview(&mut self) {
        self.request_preview_of(Some(self.chord_cursor()));
    }

    fn request_preview_of(&mut self, chord_index: Option<usize>) {
        let voicing_context = self.preview_voicing_context();
        let request = match self.preview_target() {
            Some(section) => {
                let mut request = PreviewRequest::section(
                    section.name.clone(),
                    section.degrees.clone(),
                    chord_index,
                );
                request.voicing_context = voicing_context;
                request
            }
            None => PreviewRequest::silent(),
        };
        self.pending_preview = Some(request);
    }

    fn preview_voicing_context(&self) -> PreviewVoicingContext {
        match self.focus {
            Pane::Sections => PreviewVoicingContext {
                progressions: self
                    .selected_section()
                    .map(|section| vec![section.degrees.clone()])
                    .unwrap_or_default(),
                selected: 0,
            },
            Pane::Arrangement => PreviewVoicingContext {
                progressions: self
                    .song
                    .arrangement
                    .iter()
                    .map(|id| {
                        self.song
                            .section(*id)
                            .map(|section| section.degrees.clone())
                            .unwrap_or_default()
                    })
                    .collect(),
                selected: self.clamped_arrangement_cursor(),
            },
        }
    }

    /// カーソルが指している section。右 pane では**参照先**の section
    /// （並びの行そのものは進行を持たない）。参照が壊れていれば `None`。
    pub(super) fn preview_target(&self) -> Option<&Section> {
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

    /// 現在の pane / cursor が試聴対象として指している section。
    ///
    /// host が Chord Chart から音色 selector を直接開くとき、同じ進行を候補音色で
    /// 試聴するために使う。
    pub fn selected_preview_section(&self) -> Option<&Section> {
        self.preview_target()
    }
}

#[cfg(test)]
mod tests;
