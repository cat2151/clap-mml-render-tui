//! 先読みスケジューリングから実発音表示を分離するスナップショット。
//!
//! MIDI は数ステップ先まで組み立てるため、スケジューリング側の chord / grid / bank は
//! 実際の発音より先に進む。描画まで同じ状態を見ると、再生ヘッドだけ旧 loop に残って
//! 内容が次 loop へ先行する。このモジュールは、各ステップを組み立てた時点の演奏内容を
//! 締切まで待たせ、耳に聞こえる内容と画面を揃える。

use std::time::Instant;

use super::{
    chord::resolved_note_from, ChordPlayback, DrawnPhrases, GridInstance, GridState, LaneAddress,
    VisibleNoteRow,
};

#[derive(Clone, Debug)]
pub(super) struct GridPresentation {
    instances: Vec<GridInstance>,
    chord: Option<ChordPlayback>,
    drawn: DrawnPhrases,
    bank: usize,
}

#[derive(Clone, Debug)]
pub(super) struct PendingDisplay {
    pub(super) deadline: Instant,
    pub(super) step: usize,
    pub(super) presentation: GridPresentation,
}

impl GridState {
    pub(super) fn capture_presentation(&self) -> GridPresentation {
        GridPresentation {
            instances: self.instances.clone(),
            chord: self.chord.clone(),
            drawn: self.drawn,
            bank: self.bank,
        }
    }

    pub(crate) fn display_instances(&self) -> &[GridInstance] {
        self.display
            .as_ref()
            .map_or(self.instances.as_slice(), |display| {
                display.instances.as_slice()
            })
    }

    pub(crate) fn display_chord(&self) -> Option<&ChordPlayback> {
        self.display
            .as_ref()
            .map_or(self.chord.as_ref(), |display| display.chord.as_ref())
    }

    pub(crate) fn display_visible_note_rows(&self) -> Vec<VisibleNoteRow> {
        Self::visible_note_rows_from(self.display_instances(), self.display_chord().is_some())
    }

    pub(crate) fn display_resolved_note(&self, address: LaneAddress) -> Option<u8> {
        resolved_note_from(self.display_instances(), self.display_chord(), address)
    }

    pub(crate) fn display_instance_id(&self, instance: usize) -> u8 {
        let bank = self
            .display
            .as_ref()
            .map_or(self.bank, |display| display.bank);
        (bank * self.instances.len() + instance) as u8
    }

    pub(crate) fn displayed_drawn_phrases(&self) -> DrawnPhrases {
        self.display
            .as_ref()
            .map_or(self.drawn, |display| display.drawn)
    }

    /// 手操作で選んだ型は即座に聞かせる経路なので、現在値と表示値を一緒に更新する。
    pub(crate) fn display_drawn_now(&mut self, drawn: DrawnPhrases) {
        self.drawn = self.drawn.merged(drawn);
        if let Some(display) = &mut self.display {
            display.drawn = display.drawn.merged(drawn);
        }
    }

    pub(crate) fn clear_displayed_arp(&mut self) {
        self.drawn.arp = None;
        if let Some(display) = &mut self.display {
            display.drawn.arp = None;
        }
    }

    pub(crate) fn clear_displayed_bass(&mut self) {
        self.drawn.bass = None;
        if let Some(display) = &mut self.display {
            display.drawn.bass = None;
        }
    }

    pub(crate) fn clear_displayed_drums(&mut self) {
        self.drawn.clear_drums();
        if let Some(display) = &mut self.display {
            display.drawn.clear_drums();
        }
    }
}
