//! raw / Articulated のイベント列の pane。1 行 1 イベントで「秒・種類・中身・値」。

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_DARK_GRAY, MONOKAI_PINK, MONOKAI_YELLOW},
};

use super::{note_name, pane_block};
use crate::humanize::PICKING_CC;
use crate::release::{release_shape_name, RELEASE_LEVEL_CC, RELEASE_SHAPE_CC};
use crate::{
    keyswitch_name, Articulation, GuitarArticulationScreen, Take, TimedMidiEvent, LONG_EXTRA_CC,
    POWER_CHORD_CC, SLIDE_IN_WIDTH_CC, SLIDE_WIDTH_CC, VIBRATO_DEPTH_CC,
};

const PITCH_BEND: u8 = 0xE0;
const PITCH_BEND_CENTER: i32 = 8192;
/// いちばん長い中身の列と値の間の空白。
const VALUE_GAP: usize = 5;
/// この velocity 以上の演奏音の値を目立たせる。
const LOUD_VELOCITY: u8 = 110;
/// note off の値の欄。
const OFF_TEXT: &str = "off";

/// 1 イベントの表示。
pub(crate) struct EventRow {
    seconds: f64,
    kind: &'static str,
    name: String,
    value: String,
    tone: Tone,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    Plain,
    /// [`LOUD_VELOCITY`] 以上の演奏音。
    Loud,
    NoteOff,
}

impl EventRow {
    /// `name_keyswitches` なら KS の音高を奏法の名前で出す。
    pub(crate) fn new(event: &TimedMidiEvent, name_keyswitches: bool) -> Self {
        let [status, data1, data2] = event.message;
        let keyswitch = Articulation::from_keyswitch(data1)
            .map(Articulation::name)
            .or_else(|| keyswitch_name(data1))
            .filter(|_| name_keyswitches);
        let note = || match keyswitch {
            Some(name) => format!("KS {name}"),
            None => note_name(data1),
        };
        let (kind, name, value, tone) = match status & 0xF0 {
            0x90 if data2 != 0 => {
                let loud = keyswitch.is_none() && data2 >= LOUD_VELOCITY;
                let tone = if loud { Tone::Loud } else { Tone::Plain };
                ("on", note(), data2.to_string(), tone)
            }
            0x80 | 0x90 => ("off", note(), OFF_TEXT.to_string(), Tone::NoteOff),
            0xB0 => (
                "cc",
                control_name(data1),
                control_value(data1, data2),
                Tone::Plain,
            ),
            PITCH_BEND => (
                "pb",
                String::new(),
                pitch_bend_value(data1, data2).to_string(),
                Tone::Plain,
            ),
            _ => (
                "?",
                format!("{status:02X} {data1:02X}"),
                data2.to_string(),
                Tone::Plain,
            ),
        };
        EventRow {
            seconds: event.seconds,
            kind,
            name,
            value,
            tone,
        }
    }

    /// `name_width` は列の中でいちばん長い中身の桁（[`name_width`]）。
    pub(crate) fn text(&self, name_width: usize) -> String {
        format!("{}{:>3}", self.head(name_width), self.value)
    }

    fn head(&self, name_width: usize) -> String {
        format!(
            "{:>7.3} {:<3} {:<name_width$}{:VALUE_GAP$}",
            self.seconds, self.kind, self.name, ""
        )
    }

    fn line(&self, name_width: usize) -> Line<'static> {
        let value = format!("{:>3}", self.value);
        match self.tone {
            Tone::NoteOff => {
                Line::styled(self.text(name_width), base_style().fg(MONOKAI_DARK_GRAY))
            }
            Tone::Loud => Line::from(vec![
                Span::raw(self.head(name_width)),
                Span::styled(value, Style::default().fg(MONOKAI_PINK)),
            ]),
            Tone::Plain => Line::from(self.text(name_width)),
        }
    }
}

/// 列の中でいちばん長い中身の桁。
pub(crate) fn name_width(rows: &[EventRow]) -> usize {
    rows.iter()
        .map(|row| row.name.chars().count())
        .max()
        .unwrap_or_default()
}

/// METAL-GTX の CC の中身。知らない番号は番号のまま。
fn control_name(controller: u8) -> String {
    let name = match controller {
        VIBRATO_DEPTH_CC => "vibrato depth",
        SLIDE_WIDTH_CC => "slide up/down range",
        PICKING_CC => "picking noise",
        31 => "picking micro noise",
        RELEASE_SHAPE_CC => "release type",
        RELEASE_LEVEL_CC => "release volume",
        LONG_EXTRA_CC => "long/extra",
        POWER_CHORD_CC => "power chord",
        SLIDE_IN_WIDTH_CC => "slide-in range",
        _ => match crate::param_of(controller) {
            Some(param) => param.name,
            None => return format!("CC{controller}"),
        },
    };
    name.to_string()
}

/// pitch bend の中央からの差（-8192〜+8191）。
fn pitch_bend_value(lsb: u8, msb: u8) -> i32 {
    ((i32::from(msb) << 7) | i32::from(lsb)) - PITCH_BEND_CENTER
}

/// CC の値。リリース層の種類は種類の名前を前に付ける。
fn control_value(controller: u8, value: u8) -> String {
    match controller {
        RELEASE_SHAPE_CC => format!("{} {value}", release_shape_name(value)),
        _ => value.to_string(),
    }
}

pub(super) fn draw(
    f: &mut Frame<'_>,
    area: Rect,
    title: &'static str,
    screen: &GuitarArticulationScreen,
    take: Take,
) {
    let block = pane_block(title);
    let height = block.inner(area).height as usize;
    // 1 音モードの右 pane は、鳴らすカーソル列のイベント列だけを出す。
    let column_events;
    // 全体の演奏中は鳴っている列を真ん中に、それ以外はカーソル列を上から 1/3 に置く。
    let (events, target, lead) = if take == Take::Converted && screen.note_preview() {
        column_events = screen.column_events(take);
        let target = first_note_on(&column_events);
        (column_events.as_slice(), target, height / 3)
    } else {
        let events = screen.events(take);
        match screen.playhead_row(take) {
            Some(row) => (events, Some(row), height / 2),
            None => (events, cursor_event_index(screen, events, take), height / 3),
        }
    };
    let rows: Vec<EventRow> = events
        .iter()
        .map(|event| EventRow::new(event, take == Take::Converted))
        .collect();
    f.render_widget(
        Paragraph::new(visible_lines(&rows, height, target, lead)).block(block),
        area,
    );
}

/// `height` 行に収まる分の行。`target` の行を黄色にし、その上に `lead` 行が来るよう送る。
pub(super) fn visible_lines(
    rows: &[EventRow],
    height: usize,
    target: Option<usize>,
    lead: usize,
) -> Vec<Line<'static>> {
    let width = name_width(rows);
    let first = target.map_or(0, |index| index.saturating_sub(lead));
    rows.iter()
        .enumerate()
        .skip(first)
        .take(height)
        .map(|(index, row)| {
            let line = row.line(width);
            if Some(index) == target {
                line.style(base_style().fg(MONOKAI_YELLOW))
            } else {
                line
            }
        })
        .collect()
}

/// カーソル列でいちばん早く鳴る note on（同時刻の KS を含む）の位置。
fn cursor_event_index(
    screen: &GuitarArticulationScreen,
    events: &[TimedMidiEvent],
    take: Take,
) -> Option<usize> {
    let seconds = screen.column_on_seconds(screen.cursor(), take)?;
    events
        .iter()
        .position(|event| is_note_on(event) && (event.seconds - seconds).abs() < 1e-9)
}

/// いちばん早い note on の位置。
pub(super) fn first_note_on(events: &[TimedMidiEvent]) -> Option<usize> {
    events.iter().position(is_note_on)
}

fn is_note_on(event: &TimedMidiEvent) -> bool {
    event.message[0] & 0xF0 == 0x90 && event.message[2] != 0
}
