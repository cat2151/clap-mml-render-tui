use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_patch_select::ui::{draw_patch_select_in, PatchSelectDrawOptions};
use cmrt_tui_core::theme::{MONOKAI_DARK_GRAY, MONOKAI_YELLOW};

use crate::NotepadScreen;

use super::super::{
    cache_marker, mml_cache_hit,
    status::{base_style, keybind_text, render_status_color, render_status_text},
    Mode,
};

/// これ以下の振幅しか出ない preview は無音とみなす（-80 dBFS）。
const SILENT_PEAK: f32 = 1.0e-4;
const SILENT_MARK: &str = "無音";

fn is_silent(samples: &[f32]) -> bool {
    samples.iter().all(|sample| sample.abs() <= SILENT_PEAK)
}

/// render 済みでも無音の preview は、印を「無音」にして鳴らない理由を切り分けられるようにする。
/// サンプルがメモリに無いディスク上の wav は読まずに通常の印で出す。
fn preview_marker(
    cache: &std::collections::HashMap<String, Vec<f32>>,
    cached: bool,
    render_status: Option<crate::render_queue::TuiRenderJobStatus>,
    mml: &str,
) -> Span<'static> {
    let silent = cached
        && cache
            .get(mml.trim())
            .is_some_and(|samples| is_silent(samples));
    if silent {
        return Span::styled(SILENT_MARK, Style::default().fg(MONOKAI_DARK_GRAY));
    }
    Span::raw(cache_marker(cached, render_status))
}

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
                return Span::raw(cache_marker(false, None));
            };
            let cached = mml_cache_hit(&cache, &disk_hashes, &mml);
            let render_status = (!cached)
                .then(|| app.render_job_status_for_mml(&mml))
                .flatten();
            preview_marker(&cache, cached, render_status, &mml)
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
    let heavy_note = if app.selected_heavy_sample_bytes().is_some() {
        super::heavy_preview::AUTO_PREVIEW_SKIPPED_NOTE
    } else {
        ""
    };
    let render_status_snapshot = app.render_status_snapshot();
    f.render_widget(
        Paragraph::new(format!("{status}  {selection_status}{heavy_note}"))
            .style(base_style().fg(status_color)),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(render_status_text(&render_status_snapshot))
            .style(base_style().fg(render_status_color(&render_status_snapshot))),
        chunks[2],
    );
    f.render_widget(
        Paragraph::new(keybind_text(&mode)).style(base_style()),
        chunks[3],
    );
    if let Some(preview) = &app.heavy_preview {
        super::heavy_preview::draw(preview, f);
    }
}
