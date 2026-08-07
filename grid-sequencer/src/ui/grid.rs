use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_DARK_GRAY, MONOKAI_GRAY, MONOKAI_GREEN},
};

use crate::{
    GridConnectionStatus, GridRowReadiness, GridSequencerScreen, NoteStep, VisibleNoteRow,
    VisibleRowKind, GRID_STEPS,
};

use super::layout::PATCH_WIDTH;

const NOTE_CELL: &str = "# ";
const TIE_CELL: &str = "- ";
const REST_CELL: &str = ". ";

pub(super) fn draw(
    screen: &GridSequencerScreen,
    connection: &GridConnectionStatus,
    f: &mut Frame<'_>,
    area: Rect,
) {
    let playhead = screen.state.step_index();
    let mut lines = vec![header_line()];
    lines.extend(
        screen
            .state
            .visible_note_rows()
            .into_iter()
            .map(|visible| row_line(screen, connection, visible, playhead)),
    );
    f.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Grid Sequencer / Note [{}] ",
                    screen.pattern_evolution().label()
                ))
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

fn header_line() -> Line<'static> {
    let style = base_style().fg(MONOKAI_GRAY);
    Line::from(vec![
        Span::styled(label_columns("#", "V", "PATCH", "NOTE"), style),
        Span::styled(step_ruler(), style),
    ])
}

fn row_line(
    screen: &GridSequencerScreen,
    connection: &GridConnectionStatus,
    visible: VisibleNoteRow,
    playhead: usize,
) -> Line<'static> {
    let address = visible.address;
    let instance = &screen.state.instances()[address.instance];
    let lane = &instance.lanes[address.lane];
    let readiness = connection.row_readiness(address.instance);
    let summary = visible.kind == VisibleRowKind::ChordSummary;
    let resolved = if summary {
        screen
            .state
            .chord()
            .and_then(|chord| chord.current().first().copied())
    } else {
        screen.state.resolved_note(address)
    };
    let inactive = !summary && resolved.is_none();
    let displayed_patch = screen
        .patch_selector
        .as_ref()
        .filter(|selector| selector.instance == address.instance)
        .and_then(|selector| selector.selected_patch())
        .or(instance.patch.as_deref());
    let group_header = if screen.state.chord().is_some()
        && instance.lane_mode == crate::GridLaneMode::ChordVoices4
    {
        address.lane + 1 == instance.lanes.len()
    } else {
        address.lane == 0
    };
    let instance_label = group_header.then(|| (address.instance + 1).to_string());
    let voice_label = if summary {
        "C".to_string()
    } else {
        (address.lane + 1).to_string()
    };
    let patch_label = if group_header {
        truncate_patch(displayed_patch, PATCH_WIDTH)
    } else {
        String::new()
    };
    let note_label = resolved.map_or_else(|| "--".to_string(), |note| note.to_string());
    let mut spans = vec![Span::styled(
        label_columns(
            instance_label.as_deref().unwrap_or(""),
            &voice_label,
            &patch_label,
            &note_label,
        ),
        label_style(readiness, inactive),
    )];
    for step in 0..GRID_STEPS {
        let note_step = if summary {
            if step == 0 {
                NoteStep::Attack
            } else {
                NoteStep::Tie
            }
        } else {
            lane.pattern.step(step).unwrap_or(NoteStep::Rest)
        };
        let style = cell_style(readiness, note_step, inactive);
        let style = if step == playhead {
            cursor_highlight_style(style)
        } else {
            style
        };
        let symbol = match note_step {
            NoteStep::Rest => REST_CELL,
            NoteStep::Attack => NOTE_CELL,
            NoteStep::Tie => TIE_CELL,
        };
        spans.push(Span::styled(symbol, style));
    }
    Line::from(spans)
}

fn label_style(readiness: GridRowReadiness, inactive: bool) -> Style {
    if inactive {
        return base_style().fg(MONOKAI_DARK_GRAY);
    }
    match readiness {
        GridRowReadiness::Prepared => base_style(),
        GridRowReadiness::InstanceReady => base_style().fg(MONOKAI_GRAY),
        GridRowReadiness::Pending => base_style().fg(MONOKAI_DARK_GRAY),
    }
}

fn cell_style(readiness: GridRowReadiness, step: NoteStep, inactive: bool) -> Style {
    let color = if inactive {
        MONOKAI_DARK_GRAY
    } else {
        match readiness {
            GridRowReadiness::Prepared if step == NoteStep::Attack => MONOKAI_GREEN,
            GridRowReadiness::Prepared if step == NoteStep::Tie => MONOKAI_CYAN,
            GridRowReadiness::Prepared | GridRowReadiness::InstanceReady => MONOKAI_GRAY,
            GridRowReadiness::Pending => MONOKAI_DARK_GRAY,
        }
    };
    base_style().fg(color)
}

fn label_columns(instance: &str, voice: &str, patch: &str, note: &str) -> String {
    format!(
        " {instance:>2} {voice:>1} {patch:<width$} {note:>4} ",
        width = PATCH_WIDTH
    )
}

fn step_ruler() -> String {
    (0..GRID_STEPS)
        .map(|step| {
            if step % 4 == 0 {
                format!("{:<2}", step + 1)
            } else {
                "  ".to_string()
            }
        })
        .collect()
}

fn truncate_patch(patch: Option<&str>, width: usize) -> String {
    let Some(patch) = patch else {
        return "-".to_string();
    };
    let chars = patch.chars().collect::<Vec<_>>();
    if chars.len() <= width {
        return patch.to_string();
    }
    let tail = chars[chars.len() + 1 - width..].iter().collect::<String>();
    format!("…{tail}")
}
