//! 演奏位置の描画を確かめる、matrix に入りきらない長さの素材。

use std::path::PathBuf;

use ratatui::buffer::Buffer;

use super::{render, screen_with_mml};
use crate::{GuitarArticulationScreen, Take, TimedMidiEvent};

/// 7 音を 20 回繰り返した 140 列。matrix に入る列数よりずっと多い。
pub(super) fn long_mml_screen() -> GuitarArticulationScreen {
    screen_with_mml(&format!("o3 l16 {}", "cdefgab".repeat(20)))
}

/// 0.1 秒おきに C4〜F#4 を順に鳴らす `count` 音の SMF 素材。
pub(super) fn long_smf_screen(count: usize) -> GuitarArticulationScreen {
    let mut screen = screen_with_mml("o3 l8 e g a");
    let events = (0..count)
        .flat_map(|i| {
            let seconds = i as f64 * 0.1;
            let pitch = 60 + (i % 7) as u8;
            [
                TimedMidiEvent {
                    seconds,
                    message: [0x90, pitch, 100],
                },
                TimedMidiEvent {
                    seconds: seconds + 0.05,
                    message: [0x80, pitch, 0],
                },
            ]
        })
        .collect();
    screen.load_smf(PathBuf::from("long.mid"), Ok(events));
    screen
}

/// 列 `column` が鳴っている最中に止めて描く。
pub(super) fn render_playing(
    screen: &mut GuitarArticulationScreen,
    take: Take,
    column: usize,
) -> Buffer {
    let on = screen.column_on_seconds(column, take).unwrap();
    screen.set_playhead(Some((take, on + 0.01)));
    assert_eq!(screen.playhead_column(), Some(column));
    render(screen)
}
