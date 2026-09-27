use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Color,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_patch_select::ui::{draw_patch_select_in, PatchSelectDrawOptions};
use cmrt_tui_core::theme::MONOKAI_YELLOW;

use crate::NotepadScreen;

use super::super::{
    cache_marker, mml_cache_hit,
    status::{base_style, keybind_text, render_status_color, render_status_text},
    Mode,
};

/// 共有の音色選択を枠の中へ描き、枠の下 3 行（status・render status・keybind）を notepad が描く。
pub(crate) fn draw_patch_select(
    app: &NotepadScreen<'_>,
    f: &mut Frame,
    status: &str,
    status_color: Color,
    mode: Mode,
) {
    let Some(select) = app.patch_select.as_ref() else {
        return;
    };
    let area = cmrt_tui_core::ui::centered_rect(88, 76, f.area());
    f.render_widget(Clear, area);
    let overlay_block = Block::default()
        .borders(Borders::ALL)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_YELLOW));
    let inner = overlay_block.inner(area);
    f.render_widget(overlay_block, area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    {
        let cache = app.audio.cache.lock().unwrap();
        let disk_hashes = app.audio.known_disk_hashes.lock().unwrap();
        let preview_mml = app.patch_select_preview_mml_builder();
        let marker = |patch_name: &str| {
            let Some(mml) = preview_mml
                .as_ref()
                .map(|preview_mml| preview_mml.for_selector_patch(select, patch_name))
            else {
                return cache_marker(false, None);
            };
            let cached = mml_cache_hit(&cache, &disk_hashes, &mml);
            let render_status = (!cached)
                .then(|| app.render_job_status_for_mml(&mml))
                .flatten();
            cache_marker(cached, render_status)
        };
        draw_patch_select_in(
            select,
            f,
            chunks[0],
            &PatchSelectDrawOptions {
                patch_marker: Some(&marker),
                show_play_settings_hint: false,
            },
        );
    }

    let selection_status = super::selection_status_text(select.cursor(), select.filtered_len());
    let render_status_snapshot = app.render_status_snapshot();
    f.render_widget(
        Paragraph::new(format!("{status}  {selection_status}"))
            .style(base_style().fg(status_color)),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(render_status_text(render_status_snapshot))
            .style(base_style().fg(render_status_color(render_status_snapshot))),
        chunks[2],
    );
    f.render_widget(
        Paragraph::new(keybind_text(&mode)).style(base_style()),
        chunks[3],
    );
}
