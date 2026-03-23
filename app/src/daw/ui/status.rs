use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

use super::{
    super::{DawApp, DawMode, DawPlayState},
    loop_status_label,
};

pub(super) fn draw_status(app: &DawApp, f: &mut Frame, play_area: Rect, footer_area: Rect) {
    // play_state と play_position を一度だけロックしてスナップショットを取る。
    let play_state = app.play_state.lock().unwrap().clone();
    let play_position = app.play_position.lock().unwrap().clone();
    let loop_label = if play_state == DawPlayState::Playing {
        let play_measure_mmls = app.play_measure_mmls.lock().unwrap();
        loop_status_label(&play_measure_mmls)
    } else {
        None
    };

    // 拍子・テンポは常に現在の app 状態から取得することで、
    // hot reload 後もビート表示が正確に保たれる。
    let beat_count = app.beat_numerator();
    let beat_duration_secs = 60.0 / app.tempo_bpm();

    let play_text = match &play_state {
        DawPlayState::Idle => String::new(),
        DawPlayState::Playing | DawPlayState::Preview => {
            let label = if play_state == DawPlayState::Preview {
                "PREVIEW".to_string()
            } else {
                loop_label.unwrap_or_else(|| "loop".to_string())
            };
            if let Some(pos) = &play_position {
                let elapsed = pos.measure_start.elapsed().as_secs_f64();
                let raw_beat = (elapsed / beat_duration_secs) as u32;
                let current_beat = (raw_beat % beat_count) + 1;
                format!(
                    "▶ meas{}, beat{} ({})",
                    pos.measure_index + 1,
                    current_beat,
                    label
                )
            } else {
                format!("▶ 演奏中 ({})", label)
            }
        }
    };

    let footer_text = match app.mode {
        DawMode::Normal => {
            "DAW  h/l:小節移動  j/k:track移動  i:INSERT  p:play/stop  r:random音色  K:ヘルプ  d/ESC:戻る  q:終了"
        }
        DawMode::Insert => "ESC:確定→NORMAL  Enter:確定→次小節",
        DawMode::Help => "HELP  ESC:キャンセル",
    };

    let play_color = match play_state {
        DawPlayState::Idle => Color::Cyan,
        DawPlayState::Playing => Color::Yellow,
        DawPlayState::Preview => Color::Magenta,
    };

    f.render_widget(
        Paragraph::new(play_text).style(Style::default().fg(play_color)),
        play_area,
    );
    f.render_widget(
        Paragraph::new(footer_text).style(Style::default().fg(Color::Cyan)),
        footer_area,
    );
}
