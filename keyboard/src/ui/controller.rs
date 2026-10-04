use ratatui::text::{Line, Span};

use super::{choice_style, shortcut_label};
use crate::{KeyboardState, ModulationMode, PitchBendMode, VelocityMode};
use cmrt_tui_core::status::base_style;

/// `p` を押すたびに巡る順。`Idle`（一度も押していない）は巡回に含まれない。
const PITCH_BEND_ORDER: [(PitchBendMode, &str); 6] = [
    (PitchBendMode::Max, "+8191"),
    (PitchBendMode::CenterAfterMax, "0"),
    (PitchBendMode::Min, "-8192"),
    (PitchBendMode::CenterAfterMin, "0"),
    (PitchBendMode::Periodic, "cyc"),
    (PitchBendMode::CenterAfterCycle, "0"),
];

/// コントローラの行。1 つの操作につき 1 行で、巡回する選択肢を全部並べ、今の選択肢に色を付ける。
pub(super) fn controller_status_lines(state: &KeyboardState) -> [Line<'static>; 4] {
    controller_lines(state, state.combo_progress())
}

/// コントローラの行の最大表示幅。tick ごとに増える Combo の引いた数は総数の桁で測るので、
/// 演奏中に値が変わっても同じ幅を返す。
pub(super) fn controller_status_width(state: &KeyboardState) -> usize {
    let combo = state.combo_progress().map(|(_, total)| (total, total));
    controller_lines(state, combo)
        .iter()
        .map(Line::width)
        .max()
        .unwrap_or(0)
}

fn controller_lines(state: &KeyboardState, combo: Option<(usize, usize)>) -> [Line<'static>; 4] {
    let velocity = state.velocity_mode();
    let periodic_velocity = format!("cyc({})", state.velocity());
    let modulation = state.modulation_mode();
    let pitch_bend = state.pitch_bend_mode();
    let cc_periodic = state.cc_periodic_on();

    let mut cc_line = shortcut_label("cc#(x)", 'x');
    cc_line.push(Span::styled(
        format!(": {}  ", state.cc_number()),
        base_style(),
    ));
    cc_line.extend(choice_line(
        "Z",
        'Z',
        [("off", !cc_periodic), ("cyc", cc_periodic)],
    ));
    if let Some((drawn, total)) = combo {
        cc_line.push(Span::styled(
            format!("  Combo: {drawn}/{total}"),
            base_style(),
        ));
    }

    [
        Line::from(choice_line(
            "vel",
            'v',
            [
                ("100", velocity == VelocityMode::Normal),
                ("127", velocity == VelocityMode::Accent),
                (
                    if velocity == VelocityMode::Periodic {
                        periodic_velocity.as_str()
                    } else {
                        "cyc"
                    },
                    velocity == VelocityMode::Periodic,
                ),
            ],
        )),
        Line::from(choice_line(
            "mod",
            'm',
            [
                ("off", modulation == ModulationMode::Off),
                ("on", modulation == ModulationMode::On),
                ("cyc", modulation == ModulationMode::Periodic),
            ],
        )),
        Line::from(choice_line(
            "pb",
            'p',
            PITCH_BEND_ORDER.map(|(mode, label)| (label, mode == pitch_bend)),
        )),
        Line::from(cc_line),
    ]
}

/// `label: 選択肢1  選択肢2 …`。`label` の中の `key` はショートカット色にする。
fn choice_line<'a>(
    label: &'static str,
    key: char,
    choices: impl IntoIterator<Item = (&'a str, bool)>,
) -> Vec<Span<'static>> {
    let mut spans = shortcut_label(label, key);
    spans.push(Span::styled(": ", base_style()));
    for (index, (choice, selected)) in choices.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("  ", base_style()));
        }
        spans.push(Span::styled(choice.to_string(), choice_style(selected)));
    }
    spans
}
