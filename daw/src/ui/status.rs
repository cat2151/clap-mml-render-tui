use cmrt_tui_core::sound_check_guide::SoundCheckGuidePresentation;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::Paragraph,
    Frame,
};

use super::{
    super::{DawApp, DawMode, DawPlayState, DawProjectFileAction, WorkspaceKind},
    loop_measure_summary_label, loop_status_label, MONOKAI_CYAN, MONOKAI_GREEN, MONOKAI_PURPLE,
    MONOKAI_YELLOW,
};
use crate::messages::project as project_message;

pub(super) fn daw_title(app: &DawApp) -> String {
    match app.workspace_kind {
        WorkspaceKind::Persistent => " [DAW] ".to_string(),
        WorkspaceKind::Daily => format!(
            " [DAILY DAW — {}] ",
            app.daily_page_date.as_deref().unwrap_or("unknown")
        ),
    }
}

pub(super) fn draw_status(
    app: &DawApp,
    f: &mut Frame,
    play_area: Rect,
    info_area: Rect,
    render_area: Rect,
    footer_area: Rect,
    active_render_count: usize,
) {
    // play_state と play_position を一度だけロックしてスナップショットを取る。
    let play_state = *app.playback.play_state.lock().unwrap();
    let play_position = app.playback.position.lock().unwrap().clone();
    let ab_repeat_state = app.ab_repeat_state();
    let (loop_label, loop_summary) = if play_state == DawPlayState::Playing {
        let play_measure_mmls = app.playback.measure_mmls.lock().unwrap();
        (
            loop_status_label(&play_measure_mmls, ab_repeat_state),
            loop_measure_summary_label(&play_measure_mmls, ab_repeat_state),
        )
    } else {
        (None, None)
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
        DawMode::Normal if app.workspace_kind == WorkspaceKind::Daily => {
            "DAILY DAW  Shift+H:history  h/←・l/→:meas preview  j/k:track preview  dd:cut  p:paste  u:undo  i/Ctrl+P:MML入力  e:cfg  g:generate  a:A-B  Shift+P:play/stop  Enter/Space:1trk  Shift+Enter:all  Shift+Space:from here  n:notepad  v:keyboard  Ctrl+G:screens"
        }
        DawMode::Normal => {
            "DAW  Shift+H:history  h/←・l/→:meas preview  j/k:track preview  dd:cut  p:paste  u:undo  i/Ctrl+P:MML入力  f:file  e:cfg  g:generate  a:A-B  Shift+P:play/stop  Enter/Space:1trk  Shift+Enter:all  Shift+Space:from here  n:notepad  v:keyboard  Ctrl+G:screens"
        }
        DawMode::Insert => "INSERT  ESC:確定→NORMAL  Enter:確定→次小節",
        DawMode::MmlOverlay => {
            "MML  Ctrl+T:音色  Ctrl+O:履歴  Ctrl+L:演奏設定  Ctrl+Space:鳴らし直す  Enter:確定→次小節  ESC:確定→閉じる"
        }
        DawMode::Help => "HELP  ESC:キャンセル",
        DawMode::Mixer => "MIXER  h/l:track移動  j/k:-/+3dB  ESC:閉じる",
        DawMode::History => {
            "HISTORY  ?:help  Enter:確定  Space:preview  ESC:閉じる  n/p/t:overlay切替  h/l・←/→:ペイン移動してpreview  j/k・↑/↓:移動してpreview"
        }
        DawMode::PatchSelect => {
            "PATCH SELECT  ?:help  /:現在pane検索  Enter:確定  Space:preview  ESC:閉じる  n/p/t:overlay切替  h/l・←/→:ペイン移動してpreview  j/k・↑/↓:移動してpreview"
        }
        DawMode::Project if app.workspace_kind == WorkspaceKind::Daily => "DAILY DAW",
        DawMode::Project => match app.overlays.project.action {
            Some(DawProjectFileAction::Open | DawProjectFileAction::OpenDailyArchive)
                if app.overlays.project.filter_active =>
            {
                project_message::STATUS_FILTER
            }
            Some(DawProjectFileAction::Open | DawProjectFileAction::OpenDailyArchive) => {
                project_message::STATUS_OPEN
            }
            Some(DawProjectFileAction::SaveAs) => {
                project_message::STATUS_SAVE_AS
            }
            None => project_message::STATUS_MENU,
        },
    };

    let play_color = match play_state {
        DawPlayState::Idle => MONOKAI_CYAN,
        DawPlayState::Playing => MONOKAI_YELLOW,
        DawPlayState::Preview => MONOKAI_PURPLE,
    };
    let render_color: Color = if active_render_count == 0 {
        MONOKAI_GREEN
    } else {
        MONOKAI_PURPLE
    };

    f.render_widget(
        Paragraph::new(play_text).style(Style::default().fg(play_color)),
        play_area,
    );
    f.render_widget(
        Paragraph::new(loop_summary.unwrap_or_default()).style(Style::default().fg(MONOKAI_YELLOW)),
        info_area,
    );
    f.render_widget(
        Paragraph::new(format!("並列render中: {active_render_count}"))
            .style(Style::default().fg(render_color)),
        render_area,
    );
    if app.mode == DawMode::Normal
        && app.sound_check_guide.presentation() == SoundCheckGuidePresentation::Footer
    {
        f.render_widget(
            Paragraph::new(Span::styled(
                super::super::DAW_SOUND_CHECK_GUIDE_MESSAGE,
                Style::default()
                    .fg(MONOKAI_YELLOW)
                    .add_modifier(Modifier::BOLD),
            )),
            footer_area,
        );
    } else {
        f.render_widget(
            Paragraph::new(footer_text).style(Style::default().fg(MONOKAI_CYAN)),
            footer_area,
        );
    }
}
