use super::*;
use crate::{ChordPreviewContext, MmlOverlaySyntax};

fn chord_overlay(preview: Option<ChordPreviewContext>) -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.set_restored_patch(Some("Pads/Chord Pad.fxp".to_string()));
    overlay.open(MmlOverlayContext {
        input_mode: MmlOverlayInputMode::SingleLine,
        syntax: MmlOverlaySyntax::Chord(preview),
        ..MmlOverlayContext::default()
    });
    overlay
}

#[test]
fn chord_input_names_the_preview_track_patch_and_language() {
    let rendered = render(&chord_overlay(Some(ChordPreviewContext {
        chord_init: "key:G".to_string(),
        track_directive: "close".to_string(),
        mml_prefix: String::new(),
        target_label: "T2".to_string(),
    })));

    assert!(rendered.contains("CHORD"), "{rendered}");
    assert!(rendered.contains("T2"), "{rendered}");
    assert!(rendered.contains("Chord Pad.fxp"), "{rendered}");
    assert!(rendered.contains("Chord"), "{rendered}");
    assert!(
        !rendered.contains("^O"),
        "chord入力にMML履歴は出さない: {rendered}"
    );
}

#[test]
fn chord_input_explains_when_no_preview_track_exists() {
    let rendered = render(&chord_overlay(None));

    assert!(rendered.contains("CHORD"), "{rendered}");
    assert!(rendered.contains("track"), "{rendered}");
}
