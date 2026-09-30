//! サンプル MID モードでは MML 側の欄が灰色の `-` になり、右 pane に MID のイベントが並ぶ。

use std::path::PathBuf;

use super::*;
use cmrt_tui_core::theme::MONOKAI_GRAY;

fn at(seconds: f64, message: [u8; 3]) -> TimedMidiEvent {
    TimedMidiEvent { seconds, message }
}

fn screen_in_midi_mode() -> GuitarArticulationScreen {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.load_sample_midi(
        "CC22_Mute_Control.mid".to_string(),
        Ok(vec![
            at(0.0, [0x90, 17, 100]),
            at(0.0, [0x90, 60, 100]),
            at(0.5, [0x80, 60, 0]),
            at(0.5, [0x80, 17, 0]),
        ]),
    );
    screen
}

/// 枠の内側の文字（空白を落とす）。
fn inner_text(buffer: &Buffer, area: Rect) -> String {
    let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 2);
    squeezed(&rows_in(buffer, inner))
}

#[test]
fn the_midi_mode_greys_out_the_mml_side_and_lists_the_midi_on_the_right() {
    let screen = screen_in_midi_mode();
    let buffer = render(&screen);
    let layout = layout_for(buffer.area, &screen);

    for area in [layout.input, layout.matrix, layout.plain] {
        assert_eq!(inner_text(&buffer, area), "-");
        let dash = rows_in(&buffer, area)
            .iter()
            .enumerate()
            .find_map(|(dy, row)| {
                row.find('-').filter(|_| dy > 0).map(|dx| {
                    (
                        area.x + row[..dx].chars().count() as u16,
                        area.y + dy as u16,
                    )
                })
            })
            .unwrap();
        assert_eq!(buffer.cell(dash).unwrap().fg, MONOKAI_GRAY);
    }
    let all = squeezed(&rows_in(&buffer, buffer.area));
    assert!(!all.contains("o3l8ef+g"), "{all}");

    let right = squeezed(&rows_in(&buffer, layout.converted));
    assert!(
        right.contains("MID:CC22_Mute_Control.mid(space)"),
        "{right}"
    );
    assert!(right.contains("KSSus_Down"), "{right}");

    let title = squeezed(&rows_in(&buffer, Rect::new(0, 0, buffer.area.width, 1)));
    assert!(title.contains("[MID]"), "{title}");
    let status = squeezed(&rows_in(&buffer, layout.status));
    assert!(status.contains("Esc:MIDを閉じる"), "{status}");
    assert!(status.contains("?:help"), "{status}");
}

#[test]
fn the_list_overlay_names_each_file() {
    let mut screen = screen_with_mml("o3 e");
    screen.open_sample_midi_list(Ok(vec![
        PathBuf::from("dir").join("CC22_Mute_Control.mid"),
        PathBuf::from("dir").join("CC26_Action_Slide.mid"),
    ]));
    let buffer = render(&screen);

    let all = squeezed(&rows_in(&buffer, buffer.area));
    assert!(all.contains("SampleMIDI"), "{all}");
    assert!(all.contains("CC22_Mute_Control.mid"), "{all}");
    assert!(all.contains("CC26_Action_Slide.mid"), "{all}");
}

#[test]
fn the_matrix_keybinds_name_o_and_still_fit_help() {
    let screen = screen_with_mml("o3 e");
    let buffer = render(&screen);
    let status = squeezed(&rows_in(&buffer, layout_for(buffer.area, &screen).status));
    assert!(status.contains("o:MID"), "{status}");
    assert!(status.ends_with("?:help"), "{status}");
}
