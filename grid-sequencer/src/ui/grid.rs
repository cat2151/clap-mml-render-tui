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

use super::layout::{GAIN_WIDTH, PATCH_WIDTH, SWING_WIDTH};

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
    // instance ごとの発音表から毎回引き直す値なので、行ごとに呼ばず1回で済ませる。
    let swings = screen.state.display_effective_swings();
    let mut lines = vec![header_line()];
    lines.extend(
        screen
            .state
            .display_visible_note_rows()
            .into_iter()
            .map(|visible| row_line(screen, connection, visible, playhead, &swings)),
    );
    f.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title(screen))
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

/// 直近に適用したアルペジオ音型とベースラインの型はタイトルへ添える。CC1 / Velocity
/// grid が小節ごとの抽選パターンを出しているのと同じ扱い。まだ一度も生成していない
/// ものは添えない。
fn title(screen: &GridSequencerScreen) -> String {
    let random = screen.cycle_random().compact_label();
    let mut title = format!(" Grid Sequencer / Note [{random}]");
    if let Some(arp) = screen.last_arp() {
        title.push_str(&format!(" arp:{}", arp.label()));
    }
    if let Some(bass) = screen.last_bass() {
        title.push_str(&format!(" bass:{}", bass.label()));
    }
    if let Some(drum) = screen.last_drum() {
        title.push_str(&format!(" {}:{}", drum.role().label(), drum.label()));
    }
    title.push(' ');
    title
}

fn header_line() -> Line<'static> {
    let style = base_style().fg(MONOKAI_GRAY);
    Line::from(vec![
        Span::styled(
            label_columns("#", "V", "PATCH", "GAIN", "NOTE", "SW"),
            style,
        ),
        Span::styled(step_ruler(), style),
    ])
}

fn row_line(
    screen: &GridSequencerScreen,
    connection: &GridConnectionStatus,
    visible: VisibleNoteRow,
    playhead: usize,
    swings: &[Option<u8>],
) -> Line<'static> {
    let address = visible.address;
    let instance = &screen.state.display_instances()[address.instance];
    let lane = &instance.lanes[address.lane];
    let readiness = connection.row_readiness(address.instance);
    let summary = visible.kind == VisibleRowKind::ChordSummary;
    let resolved = if summary {
        screen
            .state
            .display_chord()
            .and_then(|chord| chord.current().first().copied())
    } else {
        screen.state.display_resolved_note(address)
    };
    let inactive = !summary && resolved.is_none();
    let displayed_patch = screen
        .patch_selector
        .as_ref()
        .filter(|selector| selector.instance == address.instance)
        .and_then(|selector| selector.selected_patch())
        .or(instance.patch.as_deref());
    let group_header = if screen.state.display_chord().is_some()
        && instance.lane_mode.stacks_high_notes_on_top()
    {
        // 高音が上へ反転しているので、行の先頭は最終 lane。
        address.lane + 1 == instance.lanes.len()
    } else {
        address.lane == 0
    };
    let instance_label = group_header.then(|| (address.instance + 1).to_string());
    let voice_label = if summary {
        "C".to_string()
    } else if screen.state.display_chord().is_some() && address.instance == crate::BASS_ROW {
        // bass 音は B、その1オクターブ上は 8（8va）。V 欄は1文字ぶんしかない。
        if address.lane == 0 { "B" } else { "8" }.to_string()
    } else {
        (address.lane + 1).to_string()
    };
    let patch_label = if group_header {
        truncate_patch(displayed_patch, PATCH_WIDTH)
    } else {
        String::new()
    };
    // gain は instance ごとの値なので、patch 名と同じ「グループの先頭行だけ」に出す。
    let gain_label = if group_header {
        auto_gain_label(screen, connection, address.instance)
    } else {
        String::new()
    };
    // swing も instance ごと。跳ねない行（裏拍に note on が無い）は値を持っていても
    // 効かないので `-` を出す。
    let swing_label = if group_header {
        swing_label(swings, address.instance)
    } else {
        String::new()
    };
    // drum 行は音高が意味を持たない（1 instance = 1 打楽器）ので、番号ではなく役割を出す。
    let note_label = match instance.drum {
        Some(role) => role.label().to_string(),
        None => resolved.map_or_else(|| "--".to_string(), |note| note.to_string()),
    };
    let mut spans = vec![Span::styled(
        label_columns(
            instance_label.as_deref().unwrap_or(""),
            &voice_label,
            &patch_label,
            &gain_label,
            &note_label,
            &swing_label,
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

/// 情報欄（step セルの左側）の1行。**桁の唯一の出所**で、[`super::layout`] の
/// `*_START` / `LABEL_WIDTH` はこの書式を数えた値。コンパイラは整合を見ないので、
/// 書式を変えたら向こうの定数も必ず直すこと（[`tests`] が幅の一致だけは守る）。
fn label_columns(
    instance: &str,
    voice: &str,
    patch: &str,
    gain: &str,
    note: &str,
    swing: &str,
) -> String {
    format!(
        " {instance:>2} {voice:>1} {patch:<patch_width$} {gain:>gain_width$} {note:>4} \
         {swing:>swing_width$} ",
        patch_width = PATCH_WIDTH,
        gain_width = GAIN_WIDTH,
        swing_width = SWING_WIDTH,
    )
}

/// auto gain が instance へ掛けている trim。0 dB のときも `+0.0` を出す。
///
/// 「効いていて 0 dB」と「そもそも動いていない」は空欄では見分けられないが、
/// auto gain は grid sequencer の再生中つねに on なので、空欄にすると
/// 「表示が壊れている」ようにしか見えない。
fn auto_gain_label(
    screen: &GridSequencerScreen,
    connection: &GridConnectionStatus,
    row: usize,
) -> String {
    let instance_id = screen.state.display_instance_id(row);
    format!("{:+.1}", connection.instance_auto_gain_db(instance_id))
}

/// shuffle 量の百分率。50 は「跳ねる余地はあるが等分」、`-` は「跳ねようがない」。
///
/// 裏拍に note on を持たない行（表拍の四分・八分だけの行、chord mode の和音行）は
/// 値を持っていても発音位置が動かないので、数字を出すと嘘になる。
fn swing_label(swings: &[Option<u8>], instance: usize) -> String {
    match swings.get(instance).copied().flatten() {
        Some(swing) => swing.to_string(),
        None => "-".to_string(),
    }
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

#[cfg(test)]
mod tests;

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
