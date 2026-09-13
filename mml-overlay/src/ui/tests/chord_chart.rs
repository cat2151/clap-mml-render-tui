use super::*;
use crate::{ChordChartPreviewContext, MmlOverlayInputMode, MmlOverlaySyntax, SingleLineFlow};

#[test]
fn chord_chart_input_names_the_target_patch_and_modal_actions() {
    let mut overlay = MmlOverlay::default();
    overlay.set_restored_patch(Some("Pads/Chord Pad.fxp".to_string()));
    overlay.open(MmlOverlayContext {
        input_mode: MmlOverlayInputMode::SingleLine,
        single_line_flow: SingleLineFlow::Modal,
        syntax: MmlOverlaySyntax::ChordChart(ChordChartPreviewContext {
            key_token: Some("Key=G".to_string()),
        }),
        ..MmlOverlayContext::default()
    });

    let rendered = render(&overlay);
    let compact = rendered.replace(' ', "");

    assert!(rendered.contains("CHORD"), "{rendered}");
    assert!(rendered.contains("Chord Chart"), "{rendered}");
    assert!(rendered.contains("Chord Pad.fxp"), "{rendered}");
    assert!(compact.contains("^T音色"), "{rendered}");
    assert!(compact.contains("Enter:確定"), "{rendered}");
    assert!(compact.contains("Esc:破棄"), "{rendered}");
    assert!(!rendered.contains("^O"), "{rendered}");
}
