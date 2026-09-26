//! 音色 selector の試聴で、音源へ実際に何が届くか。
//!
//! action の形ではなく、[`crate::RecordingSink`] が受けた MIDI と timeline event で
//! 音高・順序・音長を見る。試聴の判断をどこへ移しても、鳴る音はこれで固定される。

use std::sync::Arc;

use super::*;
use crate::{MmlOverlaySender, RecordingSink, NOTE_OFF};

/// 音源へ届いた音。
#[derive(Debug, PartialEq)]
pub(super) enum Heard {
    /// 音色を準備しただけで何も鳴らしていない。
    Silent { patch: Option<String> },
    /// 生 MIDI の同時発音。`held` は note on から note off までに sink が見た時間。
    Notes {
        patch: Option<String>,
        pitches: Vec<u8>,
        held: Duration,
    },
    /// live timeline の行。`(音高, 開始 ms, 長さ ms)` を開始順に。
    Line {
        patch: Option<String>,
        notes: Vec<(u8, u64, u64)>,
    },
}

/// host（app 画面）と同じ規則で action を sender へ流し、音源へ届いたものを返す。
pub(super) fn hear(overlay: &MmlOverlay<'_>, action: MmlOverlayAction) -> Heard {
    let sink = Arc::new(RecordingSink::default());
    let sender = MmlOverlaySender::with_recording_sink(Arc::clone(&sink), 48_000.0);
    let (patch, notes) = match action {
        MmlOverlayAction::SetPatch { patch, notes } => (patch, notes),
        MmlOverlayAction::SavePatchFilterPresets {
            preview: Some(preview),
            ..
        } => return hear(overlay, *preview),
        MmlOverlayAction::PlayLine { patch, program } => {
            let patch = match patch {
                PatchChange::Keep => overlay.patch().map(str::to_string),
                PatchChange::Switch(patch) => patch,
            };
            let id = sender.play_line(patch.as_deref(), program);
            wait_until("行の送信", || {
                sender
                    .status()
                    .line_playback()
                    .is_some_and(|playback| playback.command_id() == id)
            });
            drop(sender);
            return Heard::Line {
                patch: last_prepared(&sink),
                notes: timeline_notes(&sink),
            };
        }
        action => panic!("音を出す action ではない: {action:?}"),
    };
    let Some(notes) = notes else {
        sender.prepare(patch.as_deref());
        wait_until("音色の準備", || !sink.prepared().is_empty());
        drop(sender);
        return Heard::Silent {
            patch: last_prepared(&sink),
        };
    };
    let pitches = notes
        .messages
        .iter()
        .map(|message| message[1])
        .collect::<Vec<_>>();
    sender.send(patch.as_deref(), notes.messages, notes.duration);
    wait_until("gate 切れの note off", || {
        sink.midi()
            .iter()
            .any(|(_, message)| message[0] == NOTE_OFF)
    });
    drop(sender);
    let midi = sink.midi();
    let on = midi
        .iter()
        .find(|(_, message)| message[0] == NOTE_ON)
        .expect("note on")
        .0;
    let off = midi
        .iter()
        .find(|(_, message)| message[0] == NOTE_OFF)
        .expect("note off")
        .0;
    Heard::Notes {
        patch: last_prepared(&sink),
        pitches,
        held: off - on,
    }
}

fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done() {
        assert!(Instant::now() < deadline, "待ちきれなかった: {what}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn last_prepared(sink: &RecordingSink) -> Option<String> {
    sink.prepared()
        .last()
        .and_then(|patch| patch.patch().map(str::to_string))
}

fn timeline_notes(sink: &RecordingSink) -> Vec<(u8, u64, u64)> {
    let events = sink.timeline_events();
    let ms = |seconds: f64| (seconds * 1000.0).round() as u64;
    let origin = events
        .iter()
        .find(|event| event.message[0] == NOTE_ON)
        .map_or(0.0, |event| event.timeline_seconds);
    events
        .iter()
        .enumerate()
        .filter(|(_, event)| event.message[0] == NOTE_ON)
        .map(|(index, event)| {
            let off = events[index + 1..]
                .iter()
                .find(|later| later.message[0] == NOTE_OFF && later.message[1] == event.message[1])
                .expect("note off");
            (
                event.message[1],
                ms(event.timeline_seconds - origin),
                ms(off.timeline_seconds - event.timeline_seconds),
            )
        })
        .collect()
}

fn catalog() -> Vec<PatchCatalogEntry> {
    ["Leads/Lead 1.fxp", "Pads/Pad 1.fxp"]
        .into_iter()
        .map(|patch| PatchCatalogEntry::from_display(patch.to_string()))
        .collect()
}

pub(super) fn opened_with_catalog() -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        patch_catalog: PatchCatalogSnapshot::Ready(catalog()),
        ..MmlOverlayContext::default()
    });
    overlay
}

pub(super) fn turn_on_repeat(overlay: &mut MmlOverlay<'_>, now: Instant) {
    overlay.handle_key(ctrl(KeyCode::Char('l')), now);
    overlay.handle_key(press(KeyCode::Char(' ')), now);
    overlay.handle_key(press(KeyCode::Enter), now);
    assert!(overlay.play_settings().repeat);
}

pub(super) const PAD: &str = "Pads/Pad 1.fxp";
pub(super) const CDE: [(u8, u64, u64); 3] = [(60, 0, 250), (62, 250, 250), (64, 500, 250)];

pub(super) fn assert_notes(heard: Heard, patch: Option<&str>, pitches: &[u8], gate_ms: u64) {
    let Heard::Notes {
        patch: heard_patch,
        pitches: heard_pitches,
        held,
    } = heard
    else {
        panic!("1 音の試聴のはず: {heard:?}");
    };
    assert_eq!(heard_patch.as_deref(), patch);
    assert_eq!(heard_pitches, pitches);
    assert!(held >= Duration::from_millis(gate_ms), "{held:?}");
}

#[test]
fn moving_the_candidate_sounds_the_note_at_the_cursor() {
    let mut overlay = opened_with_catalog();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    let action = overlay.handle_key(press(KeyCode::Down), now);

    assert_notes(hear(&overlay, action), Some(PAD), &[64], 250);
}

#[test]
fn moving_the_candidate_on_an_empty_line_sounds_the_fallback_note() {
    let mut overlay = opened_with_catalog();
    let now = Instant::now();
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    let action = overlay.handle_key(press(KeyCode::Down), now);

    assert_notes(hear(&overlay, action), Some(PAD), &[60], 250);
}

#[test]
fn moving_the_candidate_with_repeat_on_sounds_the_whole_line() {
    let mut overlay = opened_with_catalog();
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    turn_on_repeat(&mut overlay, now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    let action = overlay.handle_key(press(KeyCode::Down), now);

    let Heard::Line { patch, notes } = hear(&overlay, action) else {
        panic!("行の試聴のはず");
    };
    assert_eq!(patch.as_deref(), Some(PAD));
    assert_eq!(notes[..3], CDE);
}

#[test]
fn moving_the_candidate_off_every_unit_sounds_the_whole_line() {
    let mut overlay = opened_with_catalog();
    let now = Instant::now();
    type_chars(&mut overlay, "cde /*x*/", now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    let action = overlay.handle_key(press(KeyCode::Down), now);

    assert_eq!(
        hear(&overlay, action),
        Heard::Line {
            patch: Some(PAD.to_string()),
            notes: CDE.to_vec(),
        }
    );
}

#[test]
fn saving_a_filter_preset_sounds_the_note_at_the_cursor_even_with_repeat_on() {
    let patch = "Instruments/Violin.fxp";
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        patch_catalog: PatchCatalogSnapshot::Ready(vec![PatchCatalogEntry::from_display(
            patch.to_string(),
        )]),
        ..MmlOverlayContext::default()
    });
    let now = Instant::now();
    type_chars(&mut overlay, "cde", now);
    turn_on_repeat(&mut overlay, now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Left), now);
    overlay.handle_key(press(KeyCode::Left), now);
    for _ in 0..3 {
        overlay.handle_key(press(KeyCode::Down), now);
    }
    overlay.handle_key(press(KeyCode::Char('/')), now);
    for ch in "violin".chars() {
        overlay.handle_key(press(KeyCode::Char(ch)), now);
    }
    overlay.handle_key(press(KeyCode::Enter), now);

    let action = overlay.handle_key(press(KeyCode::Char('a')), now);

    assert_notes(hear(&overlay, action), Some(patch), &[64], 250);
}

fn chord_chart_overlay(role: Option<cmrt_patches::PatchRole>, line: &str) -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        input_mode: MmlOverlayInputMode::SingleLine,
        single_line_flow: SingleLineFlow::Modal,
        initial_text: line.to_string(),
        syntax: MmlOverlaySyntax::ChordChart(ChordChartPreviewContext {
            key_token: Some("Key=G".to_string()),
        }),
        patch_catalog: PatchCatalogSnapshot::Ready(
            ["Basses/Bass 1.fxp", "Basses/Bass 2.fxp"]
                .into_iter()
                .map(|patch| PatchCatalogEntry::from_display(patch.to_string()))
                .collect(),
        ),
        patch_select_initial_role: role,
        ..MmlOverlayContext::default()
    });
    overlay.request_patch_select();
    overlay
}

#[test]
fn a_chord_chart_selector_on_an_empty_line_only_prepares_the_patch() {
    let mut overlay = chord_chart_overlay(None, "");
    let now = Instant::now();

    let action = overlay.handle_key(press(KeyCode::Down), now);

    assert_eq!(
        hear(&overlay, action),
        Heard::Silent {
            patch: Some("Basses/Bass 2.fxp".to_string()),
        }
    );
}
