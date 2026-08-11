//! Grid Sequencer の mouse gesture と hit result の適用。

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::{
    ui::layout::GridHit, GridSequencerContext, GridSequencerScreen, LaneAddress, NotePattern,
    NoteStep, PatternEvolution, PitchDirection,
};

/// 並んだ list を送る向き。
///
/// 音高を上下させる [`PitchDirection`] とは別物として扱う。同じ wheel でも、対象が
/// 「値」なのか「list」なのかで下へ回したときの意味が逆になるため。
///
/// - 値（NOTE 欄の音高・転回）は上へ回せば上がる。piano roll と同じ空間の上下。
/// - list（PATCH 欄の音色・step セルのフレーズ型）はブラウザのスクロールと同じで、
///   下へ回すと次へ進む。
///
/// この2つを1つの型で兼用していたせいで、step セルと PATCH 欄だけ送りが逆になっていた。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListDirection {
    Next,
    Prev,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NoteGestureMode {
    DrawSpan,
    EraseNotes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NoteGesture {
    button: MouseButton,
    mode: NoteGestureMode,
    lanes: Vec<GestureLane>,
    last: (LaneAddress, usize),
    changed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GestureLane {
    address: LaneAddress,
    anchor: usize,
    endpoint: usize,
    before: NotePattern,
}

impl GridSequencerScreen {
    /// 現在の端末矩形で hit test し、mouse gesture を1 eventぶん進める。
    pub fn handle_mouse(
        &mut self,
        event: MouseEvent,
        terminal_area: Rect,
        ctx: &GridSequencerContext<'_>,
    ) {
        if self.patch_selector.is_some() {
            if self.patch_selector_input_enabled() {
                self.handle_patch_selector_mouse(event, terminal_area, ctx);
            } else {
                self.cancel_mouse_gesture();
            }
            return;
        }
        if !self.mouse_edit_enabled() {
            self.cancel_mouse_gesture();
            return;
        }
        let visible_rows = self.state.visible_note_rows();
        let layout = crate::ui::layout_for(self, terminal_area);
        let hit = layout.hit_test(event.column, event.row, &visible_rows);
        match event.kind {
            MouseEventKind::Down(MouseButton::Left | MouseButton::Right) => {
                self.begin_cell_gesture(event.kind, hit, ctx)
            }
            MouseEventKind::Down(_) => self.cancel_mouse_gesture(),
            MouseEventKind::Drag(button) => self.drag_note_gesture(button, hit, &visible_rows),
            MouseEventKind::Up(_) => self.finish_cell_gesture(),
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                self.handle_wheel(event.kind, event.modifiers, hit, ctx)
            }
            MouseEventKind::Moved | MouseEventKind::ScrollLeft | MouseEventKind::ScrollRight => {}
        }
    }

    pub fn cancel_mouse_gesture(&mut self) {
        self.finish_cell_gesture();
        self.cancel_patch_selector();
    }

    fn finish_cell_gesture(&mut self) {
        let changed = self
            .note_gesture
            .take()
            .is_some_and(|gesture| gesture.changed);
        self.finish_undo_gesture(changed);
    }

    pub(crate) fn begin_manual_edit(&mut self) {
        self.set_pattern_evolution(PatternEvolution::Hold);
        // HOLD 中にも、編集前のinstancesから作ったpending cycleが存在し得る。
        self.cancel_cycle_swap_preserving_drain();
    }

    pub(crate) fn set_pattern_evolution(&mut self, next: PatternEvolution) {
        if self.pattern_evolution == next {
            return;
        }
        self.pattern_evolution = next;
        // instance 1の増幅は完全ランダム(AUTO)限定。音色ロードなしでも即時反映する。
        self.apply_chord_gains();
    }

    pub(crate) fn toggle_pattern_evolution(&mut self) {
        let undo = self.capture_undo();
        let next = match self.pattern_evolution {
            PatternEvolution::Auto => {
                self.cancel_cycle_swap_preserving_drain();
                PatternEvolution::Hold
            }
            PatternEvolution::Hold => PatternEvolution::Auto,
        };
        self.set_pattern_evolution(next);
        self.commit_undo(undo);
    }

    pub(crate) fn mouse_edit_enabled(&self) -> bool {
        let connection = self.connection_status();
        self.patch_selector_input_enabled() && !connection.row_patch_is_loading()
    }

    pub(crate) fn patch_selector_input_enabled(&self) -> bool {
        let connection = self.connection_status();
        !self.help_open
            && !self.restart_notice_open()
            && !self.waiting_for_patches
            && !connection.is_preparing()
            && connection.error_message().is_none()
    }

    fn begin_cell_gesture(
        &mut self,
        kind: MouseEventKind,
        hit: Option<GridHit>,
        ctx: &GridSequencerContext<'_>,
    ) {
        self.finish_cell_gesture();
        let Some(GridHit::NoteCell { address, step }) = hit else {
            if matches!(kind, MouseEventKind::Down(MouseButton::Left)) {
                self.handle_left_click(hit, ctx);
            }
            return;
        };
        let button = match kind {
            MouseEventKind::Down(button @ (MouseButton::Left | MouseButton::Right)) => button,
            _ => return,
        };
        if self.state.chord().is_some() && address.instance == crate::CHORD_ROW {
            return;
        }
        self.begin_undo_gesture();
        let before = self
            .state
            .lane(address)
            .expect("visible row points at a stored lane")
            .pattern
            .clone();
        let mode = match button {
            MouseButton::Left if before.step(step) == Some(NoteStep::Rest) => {
                NoteGestureMode::DrawSpan
            }
            MouseButton::Left | MouseButton::Right => NoteGestureMode::EraseNotes,
            MouseButton::Middle => return,
        };
        let changed = match mode {
            NoteGestureMode::DrawSpan => self.state.draw_note_span(address, step, step),
            NoteGestureMode::EraseNotes => self.state.erase_note_at(address, step),
        };
        if changed {
            self.begin_manual_edit();
        }
        self.note_gesture = Some(NoteGesture {
            button,
            mode,
            lanes: vec![GestureLane {
                address,
                anchor: step,
                endpoint: step,
                before,
            }],
            last: (address, step),
            changed,
        });
    }

    fn handle_left_click(&mut self, hit: Option<GridHit>, ctx: &GridSequencerContext<'_>) {
        match hit {
            Some(GridHit::InstancePatch { instance }) => self.open_patch_selector(instance, ctx),
            Some(GridHit::NoteCell { .. } | GridHit::LaneNote { .. }) | None => {}
        }
    }

    fn drag_note_gesture(
        &mut self,
        button: MouseButton,
        hit: Option<GridHit>,
        visible_rows: &[crate::VisibleNoteRow],
    ) {
        let Some(mut gesture) = self.note_gesture.take() else {
            return;
        };
        if gesture.button != button {
            self.note_gesture = Some(gesture);
            return;
        }
        let Some(GridHit::NoteCell { address, step }) = hit else {
            self.note_gesture = Some(gesture);
            return;
        };
        if gesture.last == (address, step) {
            self.note_gesture = Some(gesture);
            return;
        }
        let points = drag_points(gesture.last, (address, step), visible_rows);
        gesture.last = (address, step);
        for (address, step) in points {
            let changed = self.apply_gesture_point(&mut gesture, address, step);
            if changed {
                if !gesture.changed {
                    self.begin_manual_edit();
                }
                gesture.changed = true;
            }
        }
        self.note_gesture = Some(gesture);
    }

    fn apply_gesture_point(
        &mut self,
        gesture: &mut NoteGesture,
        address: LaneAddress,
        step: usize,
    ) -> bool {
        if gesture.mode == NoteGestureMode::EraseNotes {
            return self.state.erase_note_at(address, step);
        }
        let lane_index = gesture
            .lanes
            .iter()
            .position(|lane| lane.address == address);
        let Some(lane_index) = lane_index else {
            let Some(before) = self.state.lane(address).map(|lane| lane.pattern.clone()) else {
                return false;
            };
            let changed = self.state.draw_note_span(address, step, step);
            gesture.lanes.push(GestureLane {
                address,
                anchor: step,
                endpoint: step,
                before,
            });
            return changed;
        };
        let lane = &mut gesture.lanes[lane_index];
        let endpoint = step.max(lane.anchor);
        if endpoint == lane.endpoint {
            return false;
        }
        lane.endpoint = endpoint;
        let mut pattern = lane.before.clone();
        let _ = pattern.draw_span(lane.anchor, endpoint);
        self.state.restore_note_pattern(address, pattern)
    }

    fn handle_wheel(
        &mut self,
        kind: MouseEventKind,
        modifiers: KeyModifiers,
        hit: Option<GridHit>,
        ctx: &GridSequencerContext<'_>,
    ) {
        // 同じ wheel でも、送る対象が list か値かで下の意味が変わる。両方の対応をここに
        // 並べて、どちらのモデルで動く欄なのかを対象ごとに選ぶ。
        let (list, direction) = match kind {
            MouseEventKind::ScrollDown => (ListDirection::Next, PitchDirection::Down),
            MouseEventKind::ScrollUp => (ListDirection::Prev, PitchDirection::Up),
            _ => return,
        };
        // PATCH 欄の wheel は音色の list 送り。下で次、上で前に聴いた音色へ戻る。
        if let Some(GridHit::InstancePatch { instance }) = hit {
            self.cycle_instance_patch(instance, list, ctx);
            return;
        }
        // step セル上の wheel は音高ではなく、その instance のフレーズを引き直す。
        // 行の役割ごとに引くものが違う（bass 行はベースライン、drum 行はリズム、
        // それ以外はアルペジオ）。どれも list 送り。
        if let Some(GridHit::NoteCell { address, .. }) = hit {
            if address.instance == crate::BASS_ROW {
                self.cycle_bass_line(list);
            } else if let Some(role) = self.state.drum_role(address.instance) {
                self.cycle_drum_pattern(address.instance, role, list);
            } else {
                self.cycle_arpeggio(address.instance, list);
            }
            return;
        }
        let Some(GridHit::LaneNote { address }) = hit else {
            return;
        };
        let snapshot = self.capture_undo();
        let changed = if self.state.rotate_chord_voicing(address.instance, direction) {
            true
        } else if modifiers.contains(KeyModifiers::SHIFT) {
            self.state.move_lane_octave(address, direction)
        } else {
            self.state.move_lane_pitch(address, direction)
        };
        if changed {
            self.begin_manual_edit();
            self.commit_undo(snapshot);
        }
    }
}

/// mouse eventが中間rowを飛び越えても、直線上の全visible rowへ1点ずつ割り当てる。
fn drag_points(
    from: (LaneAddress, usize),
    to: (LaneAddress, usize),
    visible_rows: &[crate::VisibleNoteRow],
) -> Vec<(LaneAddress, usize)> {
    if from.0 == to.0 {
        return vec![to];
    }
    let Some(from_row) = visible_rows.iter().position(|row| row.address == from.0) else {
        return vec![to];
    };
    let Some(to_row) = visible_rows.iter().position(|row| row.address == to.0) else {
        return vec![to];
    };
    let distance = from_row.abs_diff(to_row);
    if distance == 0 {
        return vec![to];
    }
    let row_direction = if to_row > from_row { 1isize } else { -1 };
    let step_delta = to.1 as isize - from.1 as isize;
    (1..=distance)
        .map(|offset| {
            let row = (from_row as isize + row_direction * offset as isize) as usize;
            let numerator = step_delta * offset as isize;
            let rounded = if numerator >= 0 {
                (numerator + distance as isize / 2) / distance as isize
            } else {
                (numerator - distance as isize / 2) / distance as isize
            };
            let step = (from.1 as isize + rounded).clamp(0, (crate::GRID_STEPS - 1) as isize);
            (visible_rows[row].address, step as usize)
        })
        .collect()
}

#[cfg(test)]
mod tests;
