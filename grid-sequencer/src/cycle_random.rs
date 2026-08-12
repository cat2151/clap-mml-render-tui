//! コード進行1周ごとに「何を引き直すか」を項目別に持つ設定と、その overlay の状態。
//!
//! 以前は `PatternEvolution`（AUTO / HOLD）の2値で、patch と譜面の引き直しが1つの
//! スイッチに束ねられていた。「patch だけ毎周変える」「drum の型だけ固定する」と
//! いった中間が表現できず、しかも手編集のたびに両方まとめて止まっていた。
//!
//! ここでは対象を6項目へ分解する。項目どうしが重ならないよう、instance の引き直しは
//! **行の役割ごとにちょうど1項目へ属する**（[`crate::state::randomize`]）。
//!
//! | 項目 | 対象 |
//! |---|---|
//! | PATCH | 全行の音色 |
//! | NOTE  | 役割の無い行の譜面と音高 |
//! | DRUM  | drum 行のリズム型 |
//! | ARP   | chord mode 中の bass 行・アルペジオ行の音型 |
//! | CHORD | コード進行と Key |
//! | BPM   | テンポ（Ctrl+B で範囲を指定したときだけ実効） |
//!
//! 描画と当たり判定は [`crate::ui::cycle_random`]。

use crossterm::event::KeyCode;

use crate::{log_line, GridSequencerScreen};

/// 1周ごとの引き直しの対象。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CycleRandomItem {
    Patch,
    Note,
    Drum,
    Arp,
    Chord,
    Bpm,
}

impl CycleRandomItem {
    /// overlay に並べる順序。数字キーの並びでもある。
    pub const ALL: [CycleRandomItem; 6] = [
        Self::Patch,
        Self::Note,
        Self::Drum,
        Self::Arp,
        Self::Chord,
        Self::Bpm,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Patch => "PATCH",
            Self::Note => "NOTE",
            Self::Drum => "DRUM",
            Self::Arp => "ARP",
            Self::Chord => "CHORD",
            Self::Bpm => "BPM",
        }
    }

    /// [`CycleRandom::compact_label`] が使う1文字。ON の項目だけこの文字が出る。
    pub fn initial(self) -> char {
        match self {
            Self::Patch => 'P',
            Self::Note => 'N',
            Self::Drum => 'D',
            Self::Arp => 'A',
            Self::Chord => 'C',
            Self::Bpm => 'B',
        }
    }

    /// overlay の1行に添える、何が変わるかの説明。
    pub fn description(self) -> &'static str {
        match self {
            Self::Patch => "全行の音色",
            Self::Note => "役割の無い行の譜面・音高",
            Self::Drum => "drum 行のリズム型",
            Self::Arp => "bass 行・アルペジオ行の音型",
            Self::Chord => "コード進行と Key",
            Self::Bpm => "テンポ (Ctrl+Bで範囲)",
        }
    }
}

/// 1周ごとに引き直す対象の集合。既定は全 ON（＝旧 AUTO）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CycleRandom {
    pub patch: bool,
    pub note: bool,
    pub drum: bool,
    pub arp: bool,
    pub chord: bool,
    pub bpm: bool,
}

impl Default for CycleRandom {
    fn default() -> Self {
        Self::ALL
    }
}

impl CycleRandom {
    /// 全項目 ON。`r` キーのような「丸ごと引き直す」経路もこれを使う。
    pub const ALL: Self = Self {
        patch: true,
        note: true,
        drum: true,
        arp: true,
        chord: true,
        bpm: true,
    };

    /// 全項目 OFF。
    pub const NONE: Self = Self {
        patch: false,
        note: false,
        drum: false,
        arp: false,
        chord: false,
        bpm: false,
    };

    /// 旧 HOLD 相当。譜面と音色を据え置き、進行とテンポだけ動かす。
    /// 旧セッションの移行先でもある（[`crate::session`]）。
    pub const HOLD: Self = Self {
        patch: false,
        note: false,
        drum: false,
        arp: false,
        chord: true,
        bpm: true,
    };

    pub fn get(self, item: CycleRandomItem) -> bool {
        match item {
            CycleRandomItem::Patch => self.patch,
            CycleRandomItem::Note => self.note,
            CycleRandomItem::Drum => self.drum,
            CycleRandomItem::Arp => self.arp,
            CycleRandomItem::Chord => self.chord,
            CycleRandomItem::Bpm => self.bpm,
        }
    }

    pub fn set(&mut self, item: CycleRandomItem, on: bool) {
        match item {
            CycleRandomItem::Patch => self.patch = on,
            CycleRandomItem::Note => self.note = on,
            CycleRandomItem::Drum => self.drum = on,
            CycleRandomItem::Arp => self.arp = on,
            CycleRandomItem::Chord => self.chord = on,
            CycleRandomItem::Bpm => self.bpm = on,
        }
    }

    /// instance 群（音色・譜面）を引き直す項目がひとつでも ON か。
    /// どれも OFF なら次サイクルの抽選そのものを省ける。
    pub fn instances_change(self) -> bool {
        self.patch || self.note || self.drum || self.arp
    }

    /// ON の項目だけ頭文字を並べた6文字。OFF は `-`。例: `P-D--B`。
    ///
    /// NOTE grid のタイトルとログに出す。overlay を開かなくても現在値が読める。
    pub fn compact_label(self) -> String {
        CycleRandomItem::ALL
            .iter()
            .map(|item| if self.get(*item) { item.initial() } else { '-' })
            .collect()
    }
}

/// overlay のカーソル位置だけを持つ。設定そのものは [`CycleRandom`] 側。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CycleRandomOverlay {
    cursor: usize,
}

impl CycleRandomOverlay {
    pub(crate) fn cursor(self) -> usize {
        self.cursor
    }

    pub(crate) fn selected(self) -> CycleRandomItem {
        CycleRandomItem::ALL[self.cursor]
    }

    fn set_cursor(&mut self, index: usize) {
        if index < CycleRandomItem::ALL.len() {
            self.cursor = index;
        }
    }

    /// 上下端で止める。巡回させると、端まで来たことが分からない。
    fn move_cursor(&mut self, delta: isize) {
        let last = CycleRandomItem::ALL.len() as isize - 1;
        self.cursor = (self.cursor as isize + delta).clamp(0, last) as usize;
    }
}

impl GridSequencerScreen {
    /// 1周ごとの引き直し設定。セッションへ保存する値。
    pub fn cycle_random(&self) -> CycleRandom {
        self.cycle_random
    }

    pub fn cycle_random_open(&self) -> bool {
        self.cycle_random_overlay.is_some()
    }

    pub(crate) fn cycle_random_cursor(&self) -> Option<usize> {
        self.cycle_random_overlay.map(CycleRandomOverlay::cursor)
    }

    /// `a` キー。overlay を開く（開いていれば閉じる）。
    pub(crate) fn toggle_cycle_random_overlay(&mut self) {
        self.cancel_mouse_gesture();
        self.cycle_random_overlay = match self.cycle_random_overlay {
            Some(_) => None,
            None => Some(CycleRandomOverlay::default()),
        };
    }

    pub(crate) fn close_cycle_random_overlay(&mut self) {
        self.cycle_random_overlay = None;
    }

    /// 手編集で触った対象だけを OFF にする。
    ///
    /// 旧 `begin_manual_edit()` は AUTO/HOLD ごと倒していたので、1セル描いただけで
    /// 音色の引き直しまで止まっていた。触っていない項目は動かさない。
    pub(crate) fn begin_manual_edit(&mut self, item: CycleRandomItem) {
        self.set_cycle_random(item, false);
        // 編集前の instances から作った pending cycle は、もう古い。
        self.cancel_cycle_swap_preserving_drain();
    }

    /// 1項目を切り替える。変化があったときだけ副作用を走らせる。
    pub(crate) fn set_cycle_random(&mut self, item: CycleRandomItem, on: bool) {
        if self.cycle_random.get(item) == on {
            return;
        }
        self.cycle_random.set(item, on);
        match item {
            // 和音の増幅は「譜面ごと毎周変わる完全ランダム演奏」限定。音色ロード
            // なしで即時反映する。
            CycleRandomItem::Note => self.apply_chord_gains(),
            // 音色を据え置くと決めた以上、走っている先読み（＝差し替え先 bank への
            // 音色ロード）は意味を失う。
            CycleRandomItem::Patch if !on => self.cancel_cycle_swap_preserving_drain(),
            _ => {}
        }
        log_line(&format!(
            "grid-sequencer: cycle-random {}={on} value={}",
            item.label(),
            self.cycle_random.compact_label(),
        ));
    }

    pub(crate) fn toggle_cycle_random(&mut self, item: CycleRandomItem) {
        self.set_cycle_random(item, !self.cycle_random.get(item));
    }

    /// overlay を開いている間のキー入力。
    ///
    /// カーソルは値を取り出してから書き戻す。`self` の可変借用を握ったまま
    /// `set_cycle_random` を呼べないため。
    pub(crate) fn handle_cycle_random_key(&mut self, key: crossterm::event::KeyEvent) {
        let Some(mut overlay) = self.cycle_random_overlay else {
            return;
        };
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('a') => {
                self.close_cycle_random_overlay();
                return;
            }
            KeyCode::Up | KeyCode::Char('k') => overlay.move_cursor(-1),
            KeyCode::Down | KeyCode::Char('j') => overlay.move_cursor(1),
            KeyCode::Char(' ') | KeyCode::Enter => self.toggle_cycle_random(overlay.selected()),
            KeyCode::Char('A') => self.set_all_cycle_random(true),
            KeyCode::Char('N') => self.set_all_cycle_random(false),
            KeyCode::Char(digit @ '1'..='6') => {
                let index = digit as usize - '1' as usize;
                overlay.set_cursor(index);
                self.toggle_cycle_random(CycleRandomItem::ALL[index]);
            }
            _ => {}
        }
        self.cycle_random_overlay = Some(overlay);
    }

    /// overlay 上の click。項目の行を踏んだら切り替え、枠の外なら閉じる。
    pub(crate) fn handle_cycle_random_click(
        &mut self,
        column: u16,
        row: u16,
        area: ratatui::layout::Rect,
    ) {
        let Some(mut overlay) = self.cycle_random_overlay else {
            return;
        };
        match crate::ui::cycle_random::hit_test(area, column, row) {
            Some(Some(index)) => {
                overlay.set_cursor(index);
                self.toggle_cycle_random(CycleRandomItem::ALL[index]);
                self.cycle_random_overlay = Some(overlay);
            }
            // 枠の中の、項目でないところ。閉じない（誤爆で設定画面が消えない）。
            Some(None) => {}
            None => self.close_cycle_random_overlay(),
        }
    }

    fn set_all_cycle_random(&mut self, on: bool) {
        for item in CycleRandomItem::ALL {
            self.set_cycle_random(item, on);
        }
    }

    /// undo で設定ごと巻き戻す。項目ごとの副作用を漏らさないよう1項目ずつ通す。
    pub(crate) fn restore_cycle_random(&mut self, next: CycleRandom) {
        for item in CycleRandomItem::ALL {
            self.set_cycle_random(item, next.get(item));
        }
    }
}

#[cfg(test)]
mod tests;
