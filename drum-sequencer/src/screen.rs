//! kit の行一覧と、note number / step に結び付いた入力を保持する。
//! 行 index は note の昇順。画面には高い note を上に描くので、表示行は逆順になる。
//! 入力は kit 1 つぶんの [`PATTERN_COUNT`] 個の pattern で、編集するのは選択中の 1 つだけ。

use std::ops::Range;

use crate::pattern::{
    DrumHit, DrumPattern, DEFAULT_VELOCITY, MAX_VELOCITY, MIDI_NOTES, PATTERN_COUNT,
};

/// 4/4 の 1 小節を 16 分音符で編集する。terminal サイズでは変更しない。
pub const DRUM_STEPS: usize = 16;
/// one-shot でない note を ON にしたときの音長。
const DEFAULT_SUSTAIN_STEPS: u8 = 4;
/// 繰り返し再生と SMF の tempo。
pub const DRUM_BPM: f64 = 120.0;

#[derive(Clone, Debug)]
struct SelectedKit {
    name: String,
    /// None は旧 catalog / 抽出失敗、Some([]) は取得済みの空一覧。
    notes: Option<Vec<u8>>,
    /// 名前を得られた note だけ。note の昇順・重複なし。
    names: Vec<(u8, String)>,
    /// note off を無視して鳴りきる note。昇順・重複なし。
    one_shot: Vec<u8>,
    /// 保存から戻した名前だけで、まだ catalog と照合していない kit。
    unresolved: Option<KitResolution>,
}

/// 保存から戻した kit の、catalog との照合状態。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KitResolution {
    /// catalog の読み込みを待っている。
    Waiting,
    /// catalog に見つからない / catalog を読めない。
    Missing,
}

/// host が画面の往復中も所有し続ける状態。入力は編集のたびに、kit・pattern 番号・カーソルは
/// 終了時に host が保存する。
#[derive(Clone, Debug)]
pub struct DrumSequencerScreen {
    kit: Option<SelectedKit>,
    /// 常に [`PATTERN_COUNT`] 個。
    patterns: Vec<DrumPattern>,
    pattern: usize,
    /// host がまだ保存していない編集のある pattern。編集は選択中の 1 つにしか起きない。
    edited: Option<usize>,
    pub(crate) cursor_row: usize,
    pub(crate) cursor_step: usize,
    /// note 一覧が無く行に置けないカーソル note。一覧の分かる kit を採用したときに使う。
    pending_cursor_note: Option<u8>,
    /// 表示行（上が高い note）での viewport 先頭。
    scroll_top: usize,
    pub(crate) help_open: bool,
    /// vim 風に前置した数字。次の数字以外のキーで消費する。
    pub(crate) count: Option<usize>,
    /// host が渡す演奏位置の step。停止・準備中は None。
    playhead: Option<usize>,
    /// Space / Enter で ON にした打点。host が取り出すまで残る。
    turned_on: Option<DrumHit>,
}

impl Default for DrumSequencerScreen {
    fn default() -> Self {
        Self {
            kit: None,
            patterns: vec![DrumPattern::default(); PATTERN_COUNT],
            pattern: 0,
            edited: None,
            cursor_row: 0,
            cursor_step: 0,
            pending_cursor_note: None,
            scroll_top: 0,
            help_open: false,
            count: None,
            playhead: None,
            turned_on: None,
        }
    }
}

impl DrumSequencerScreen {
    /// catalog の一覧・構成音名・one-shot の note を採用する。入力は差し替えない（kit を
    /// 替えたら host が [`Self::set_patterns`] する）ので、非表示になった note の入力も残る。
    /// カーソルの note が残れば維持し、消えたら最も低い note へ移る。step は維持する。
    pub fn set_kit(
        &mut self,
        name: String,
        notes: Option<Vec<u8>>,
        mut names: Vec<(u8, String)>,
        mut one_shot: Vec<u8>,
    ) {
        let previous_note = self.saved_cursor_note();
        let notes = notes.map(|mut notes| {
            notes.retain(|note| usize::from(*note) < MIDI_NOTES);
            notes.sort_unstable();
            notes.dedup();
            notes
        });
        names.sort_by_key(|(note, _)| *note);
        names.dedup_by_key(|(note, _)| *note);
        one_shot.sort_unstable();
        one_shot.dedup();
        self.kit = Some(SelectedKit {
            name,
            notes,
            names,
            one_shot,
            unresolved: None,
        });
        self.pending_cursor_note = self.notes().is_empty().then_some(previous_note).flatten();
        match previous_note.and_then(|note| self.notes().binary_search(&note).ok()) {
            Some(row) => self.cursor_row = row,
            None => {
                self.cursor_row = 0;
                // 最も低い note は最下行。viewport の下端へ寄せる（visible_rows が丸める）。
                self.scroll_top = usize::MAX;
            }
        }
    }

    /// 保存した kit の入力と、pattern 番号・カーソルを戻す。kit は名前だけで、host が catalog と
    /// 照合して [`Self::set_kit`] するまで行は無い。範囲外の pattern 番号・step は丸める。
    pub fn restore(
        &mut self,
        kit: Option<String>,
        patterns: impl IntoIterator<Item = (usize, DrumPattern)>,
        pattern: usize,
        cursor_note: Option<u8>,
        cursor_step: usize,
    ) {
        self.kit = kit.map(|name| SelectedKit {
            name,
            notes: None,
            names: Vec::new(),
            one_shot: Vec::new(),
            unresolved: Some(KitResolution::Waiting),
        });
        self.set_patterns(patterns);
        self.pattern = pattern.min(PATTERN_COUNT - 1);
        self.cursor_row = 0;
        self.pending_cursor_note = cursor_note.filter(|note| usize::from(*note) < MIDI_NOTES);
        self.cursor_step = cursor_step.min(DRUM_STEPS - 1);
        self.scroll_top = usize::MAX;
    }

    /// 保存から戻した kit の照合状態。catalog の kit を採用済み・未選択なら None。
    pub fn kit_resolution(&self) -> Option<KitResolution> {
        self.kit.as_ref().and_then(|kit| kit.unresolved)
    }

    /// 照合待ちの kit が catalog に無かった。名前と入力は残し、別 kit の選択を待つ。
    pub fn mark_kit_missing(&mut self) {
        if let Some(kit) = self.kit.as_mut().filter(|kit| kit.unresolved.is_some()) {
            kit.unresolved = Some(KitResolution::Missing);
        }
    }

    /// kit の入力を、保存から読んだ pattern で丸ごと置き換える。無い番号は空、範囲外の番号は捨てる。
    /// 置き換えは編集ではないので保存を求めない。
    pub fn set_patterns(&mut self, patterns: impl IntoIterator<Item = (usize, DrumPattern)>) {
        self.patterns = vec![DrumPattern::default(); PATTERN_COUNT];
        for (index, pattern) in patterns {
            if let Some(slot) = self.patterns.get_mut(index) {
                *slot = pattern;
            }
        }
        self.edited = None;
    }

    /// 編集中の pattern 番号（0 始まり）。
    pub fn pattern_index(&self) -> usize {
        self.pattern
    }

    pub fn pattern_at(&self, index: usize) -> Option<&DrumPattern> {
        self.patterns.get(index)
    }

    pub fn current_pattern(&self) -> &DrumPattern {
        &self.patterns[self.pattern]
    }

    /// 前回から編集のあった pattern 番号を返し、保存済みとして数える。
    pub fn take_edited_pattern(&mut self) -> Option<usize> {
        self.edited.take()
    }

    /// 保存するカーソル note。行の無い kit でも、戻したカーソルを失わない。
    pub fn saved_cursor_note(&self) -> Option<u8> {
        self.cursor_note().or(self.pending_cursor_note)
    }

    /// catalog から得た構成音名。名前の無い note は None。
    pub fn note_name(&self, note: u8) -> Option<&str> {
        let names = &self.kit.as_ref()?.names;
        let index = names.binary_search_by_key(&note, |(n, _)| *n).ok()?;
        Some(names[index].1.as_str())
    }

    /// 演奏位置。範囲外の step は表示しない。
    pub fn set_playhead(&mut self, step: Option<usize>) {
        self.playhead = step.filter(|step| *step < DRUM_STEPS);
    }

    pub fn playhead(&self) -> Option<usize> {
        self.playhead
    }

    pub fn help_open(&self) -> bool {
        self.help_open
    }

    pub fn kit_name(&self) -> Option<&str> {
        self.kit.as_ref().map(|kit| kit.name.as_str())
    }

    /// 現在の kit の昇順・重複なしの行一覧。
    pub fn notes(&self) -> &[u8] {
        self.kit
            .as_ref()
            .and_then(|kit| kit.notes.as_deref())
            .unwrap_or(&[])
    }

    pub fn notes_known(&self) -> bool {
        self.kit.as_ref().is_some_and(|kit| kit.notes.is_some())
    }

    pub fn cursor_note(&self) -> Option<u8> {
        self.notes().get(self.cursor_row).copied()
    }

    /// 0..15 の内部 step index。
    pub fn cursor_step(&self) -> usize {
        self.cursor_step
    }

    /// 編集中の pattern のセルが ON か。
    pub fn cell_on(&self, note: u8, step: usize) -> bool {
        self.cell_length(note, step).is_some()
    }

    /// 編集中の pattern の、ON のセルの音長（step 数）。
    pub fn cell_length(&self, note: u8, step: usize) -> Option<u8> {
        self.current_pattern().length(note, step)
    }

    /// 編集中の pattern の、ON のセルの打点。
    pub fn cell_hit(&self, note: u8, step: usize) -> Option<DrumHit> {
        self.current_pattern().hit(note, step)
    }

    /// note off を無視して鳴りきる note か。音長は鳴り方に効かない。
    pub fn is_one_shot(&self, note: u8) -> bool {
        self.kit
            .as_ref()
            .is_some_and(|kit| kit.one_shot.binary_search(&note).is_ok())
    }

    /// ON にするときの音長。one-shot は 16 分音符、それ以外は 4 分音符。
    fn default_length(&self, note: u8) -> u8 {
        if self.is_one_shot(note) {
            1
        } else {
            DEFAULT_SUSTAIN_STEPS
        }
    }

    pub(crate) fn toggle_cursor(&mut self) {
        let Some(note) = self.cursor_note() else {
            return;
        };
        let step = self.cursor_step;
        if self.cell_on(note, step) {
            self.turned_on = None;
            self.patterns[self.pattern].clear(note, step);
            self.edited = Some(self.pattern);
        } else {
            let hit = DrumHit {
                note,
                step,
                steps: self.default_length(note),
                velocity: DEFAULT_VELOCITY,
            };
            self.turned_on = Some(hit);
            self.edit(hit);
        }
    }

    /// 直前のキーで Space / Enter により ON にした打点。取り出すと空になる。
    /// pattern の切替や長さ・velocity の変更では入らない。
    pub fn take_turned_on(&mut self) -> Option<DrumHit> {
        self.turned_on.take()
    }

    /// カーソルの打点の音長を `delta` step 変える。1..=[`DRUM_STEPS`] で止まる。
    pub(crate) fn adjust_cursor_length(&mut self, delta: i8) {
        if let Some(hit) = self.snap_cursor_to_hit() {
            let steps = hit
                .steps
                .saturating_add_signed(delta)
                .clamp(1, DRUM_STEPS as u8);
            self.edit(DrumHit { steps, ..hit });
        }
    }

    /// カーソルの打点の velocity を `delta` 変える。1..=127 で止まる。
    pub(crate) fn adjust_cursor_velocity(&mut self, delta: i8) {
        if let Some(hit) = self.snap_cursor_to_hit() {
            let velocity = hit
                .velocity
                .saturating_add_signed(delta)
                .clamp(1, MAX_VELOCITY);
            self.edit(DrumHit { velocity, ..hit });
        }
    }

    /// カーソルのセルの打点。OFF なら同じ行で最も近い ON のセルへカーソルを移す（等距離は左）。
    /// 行に ON のセルが無ければ None で、カーソルは動かない。
    fn snap_cursor_to_hit(&mut self) -> Option<DrumHit> {
        let note = self.cursor_note()?;
        let cursor = self.cursor_step;
        let hit = (0..DRUM_STEPS)
            .filter_map(|step| self.current_pattern().hit(note, step))
            .min_by_key(|hit| (hit.step.abs_diff(cursor), hit.step))?;
        self.cursor_step = hit.step;
        Some(hit)
    }

    /// 値が変わらない打点は編集として数えない。
    fn edit(&mut self, hit: DrumHit) {
        let pattern = &mut self.patterns[self.pattern];
        if pattern.hit(hit.note, hit.step) != Some(hit) {
            pattern.set(hit);
            self.edited = Some(self.pattern);
        }
    }

    /// 編集する pattern を `delta` 個ずらす。端では反対の端へ回る。
    pub(crate) fn shift_pattern(&mut self, delta: isize) {
        self.pattern = (self.pattern as isize + delta).rem_euclid(PATTERN_COUNT as isize) as usize;
    }

    /// 表示行 index（0 が最上行 = 最も高い note）を行 index へ変換する。
    pub(crate) fn row_at_display(&self, display: usize) -> usize {
        self.notes().len() - 1 - display
    }

    /// 描画の高さが変わってもカーソルを見失わず、同じ viewport 内では行を動かさない。
    /// 返す範囲は表示行 index。
    pub(crate) fn visible_rows(&mut self, height: usize) -> Range<usize> {
        let len = self.notes().len();
        if height == 0 || len == 0 {
            return 0..0;
        }
        let cursor = len - 1 - self.cursor_row;
        let max_top = len.saturating_sub(height);
        self.scroll_top = self.scroll_top.min(max_top);
        if cursor < self.scroll_top {
            self.scroll_top = cursor;
        } else if cursor >= self.scroll_top + height {
            self.scroll_top = cursor + 1 - height;
        }
        self.scroll_top..(self.scroll_top + height).min(len)
    }
}

#[cfg(test)]
mod tests;
