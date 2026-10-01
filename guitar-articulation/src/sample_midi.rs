//! METAL-GTX 付属のサンプル MID（KS・CC・pitch bend 入りの完成品）を、音のまとまりごとに扱う。

use std::collections::BTreeMap;

use crate::notes::{is_note_off, is_note_on};
use crate::TimedMidiEvent;

/// 付属のサンプル MID の置き場（sforzando の `patches_dirs` からの相対。[`crate::PATCH`] と同じ書き方）。
pub const SAMPLE_MIDI_DIR: &str = "sfz/UI_METAL-GTX/Sample_MIDI_Files/Control_Change";

const PITCH_BEND: u8 = 0xE0;
const CONTROL_CHANGE: u8 = 0xB0;
/// pitch bend の中央（`E0 00 40`）。
const PITCH_BEND_CENTER: [u8; 2] = [0x00, 0x40];

/// METAL-GTX の KS 一覧（`METAL-GTX_KSMap.txt`）での名前。KS でない音高なら `None`。
pub fn keyswitch_name(pitch: u8) -> Option<&'static str> {
    const LOW: [&str; 30] = [
        "Hello!",
        "FX_Resonance",
        "FX_Slide_Noise",
        "FX_Hard_Stop",
        "Chromatic_Run",
        "Slide_FX_D",
        "Slide_FX_U",
        "Slide_FX_Wow",
        "Pick_Scratch",
        "NH",
        "PH",
        "Brush_Down",
        "Brush_Up",
        "Brush_Alt",
        "Mute_Fret_D",
        "Mute_Fret_U",
        "Mute_Fret_Alt",
        "Sus_Down",
        "Sus_Up",
        "Sus_Alt",
        "Mute_Down",
        "Mute_Up",
        "Mute_Alt",
        "Slide_Down",
        "Slide_Up",
        "Pull-Off",
        "Hammer-On",
        "Slide_In",
        "Slide_Out",
        "Pseudo_Legato",
    ];
    const HIGH_FIRST: u8 = 91;
    const HIGH: [&str; 12] = [
        "Bending_HT",
        "Bending_WT",
        "Bending_1HT",
        "Unison_Bend_Auto",
        "Unison_Bend_Manual",
        "Portament",
        "Sus_PBR12",
        "Sus_PBR24",
        "Trill_HT",
        "Trill_WT",
        "Trill_min3",
        "Trill_Maj3",
    ];
    LOW.get(pitch as usize).copied().or_else(|| {
        pitch
            .checked_sub(HIGH_FIRST)
            .and_then(|offset| HIGH.get(offset as usize).copied())
    })
}

/// METAL-GTX の KS に当たる音高か（演奏の音ではない）。
pub fn is_keyswitch_pitch(pitch: u8) -> bool {
    keyswitch_name(pitch).is_some()
}

/// 音のまとまり 1 つを、イベントから読み取った中身。matrix の 1 列に当たる。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SampleMidiGroup {
    /// 鳴る音高（低い順、重複なし）。
    pub pitches: Vec<u8>,
    /// 鳴り始めに効いている KS の音高（`sw_last` のラッチなので、それまでで最後の KS）。
    pub keyswitch: Option<u8>,
    /// 鳴っている区間（note on 〜 最も遅い note off の手前）に送られる CC の番号（小さい順、重複なし）。
    pub controllers: Vec<u8>,
    /// 鳴っている区間に pitch bend が送られるか。
    pub bend: bool,
}

/// 読み込んだサンプル MID。「音」は KS でない note on の、同じ時刻のまとまり。
#[derive(Clone, Debug, PartialEq)]
pub struct SampleMidi {
    name: String,
    events: Vec<TimedMidiEvent>,
    /// 各まとまりの最初の note on の `events` 内の位置。
    groups: Vec<usize>,
}

impl SampleMidi {
    /// `events` は時刻順（`cmrt_chord::timed_smf_events` の出力）であること。
    pub fn new(name: String, events: Vec<TimedMidiEvent>) -> Self {
        let mut groups: Vec<usize> = Vec::new();
        for (index, event) in events.iter().enumerate() {
            if !is_played_note_on(event) {
                continue;
            }
            // 秒は同じ tick から作るので、同じまとまりなら完全に一致する。
            let same = groups
                .last()
                .is_some_and(|&first| events[first].seconds == event.seconds);
            if !same {
                groups.push(index);
            }
        }
        SampleMidi {
            name,
            events,
            groups,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn events(&self) -> &[TimedMidiEvent] {
        &self.events
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// まとまり `group` の最初の note on の `events` 内の位置。
    pub fn group_event_index(&self, group: usize) -> Option<usize> {
        self.groups.get(group).copied()
    }

    /// まとまり `group` が鳴り始める秒。
    pub fn group_on_seconds(&self, group: usize) -> Option<f64> {
        self.group_event_index(group)
            .map(|index| self.events[index].seconds)
    }

    /// まとまり `group` の中身。範囲外なら `None`。
    pub fn group(&self, group: usize) -> Option<SampleMidiGroup> {
        let start = self.group_on_seconds(group)?;
        let (members, end) = self.group_members(start);
        let mut pitches: Vec<u8> = members
            .iter()
            .map(|&index| self.events[index].message)
            .filter(is_note_on)
            .map(|message| message[1])
            .collect();
        pitches.sort_unstable();
        pitches.dedup();
        let keyswitch = self
            .events
            .iter()
            .rev()
            .filter(|event| event.seconds <= start)
            .find(|event| is_keyswitch_event(event) && is_note_on(&event.message))
            .map(|event| event.message[1]);
        let during = self
            .events
            .iter()
            .filter(|event| event.seconds >= start && event.seconds < end);
        let mut controllers = Vec::new();
        let mut bend = false;
        for event in during {
            match event.message[0] & 0xF0 {
                CONTROL_CHANGE => controllers.push(event.message[1]),
                PITCH_BEND => bend = true,
                _ => {}
            }
        }
        controllers.sort_unstable();
        controllers.dedup();
        Some(SampleMidiGroup {
            pitches,
            keyswitch,
            controllers,
            bend,
        })
    }

    /// まとまり `group` だけを 0 秒から鳴らすイベント列。範囲外なら空。
    ///
    /// 先頭に、その音より前に効いている CC・pitch bend・KS を送り直し、
    /// 音の note on 〜 最も遅い note off の区間の CC・pitch bend・KS を続ける。
    /// 区間の途中で始まる別の演奏音は入れない。
    pub fn note_events(&self, group: usize) -> Vec<TimedMidiEvent> {
        let Some(start) = self.group_on_seconds(group) else {
            return Vec::new();
        };
        let (members, end) = self.group_members(start);
        let shift = |event: &TimedMidiEvent| TimedMidiEvent {
            seconds: event.seconds - start,
            message: event.message,
        };

        let mut out = self.state_before(start, end - start);
        for (index, event) in self.events.iter().enumerate() {
            if event.seconds < start || event.seconds > end {
                continue;
            }
            let keep = members.contains(&index)
                || is_controller_or_bend(event)
                || (is_keyswitch_event(event)
                    && !(is_note_on(&event.message) && event.seconds == end));
            if keep {
                out.push(shift(event));
            }
        }
        cmrt_midi_filter::sort_for_playback(&mut out);
        out
    }

    /// まとまりの note on と対応する note off の位置、と最も遅い note off の秒。
    fn group_members(&self, start: f64) -> (Vec<usize>, f64) {
        let end_of_file = self.events.last().map_or(start, |event| event.seconds);
        let mut members = Vec::new();
        let mut end = start;
        for (index, event) in self.events.iter().enumerate() {
            if !is_played_note_on(event) || event.seconds != start {
                continue;
            }
            members.push(index);
            let [status, pitch, _] = event.message;
            let off = self.events[index + 1..]
                .iter()
                .position(|later| {
                    is_note_off(&later.message)
                        && later.message[0] & 0x0F == status & 0x0F
                        && later.message[1] == pitch
                })
                .map(|offset| index + 1 + offset);
            let off_seconds = match off {
                Some(off) => {
                    members.push(off);
                    self.events[off].seconds
                }
                None => end_of_file,
            };
            end = end.max(off_seconds);
        }
        (members, end)
    }

    /// `start` より前に最後に効いた CC・pitch bend・KS を 0 秒に並べる。KS の note off は `release` 秒。
    fn state_before(&self, start: f64, release: f64) -> Vec<TimedMidiEvent> {
        let before = self.events.iter().filter(|event| event.seconds < start);
        let mut controllers: BTreeMap<(u8, u8), [u8; 3]> = BTreeMap::new();
        let mut bend: Option<[u8; 3]> = None;
        let mut keyswitch: Option<[u8; 3]> = None;
        for event in before {
            let message = event.message;
            match message[0] & 0xF0 {
                CONTROL_CHANGE => {
                    controllers.insert((message[0], message[1]), message);
                }
                PITCH_BEND => bend = Some(message),
                _ if is_keyswitch_event(event) && is_note_on(&message) => keyswitch = Some(message),
                _ => {}
            }
        }
        let bend = bend.or_else(|| {
            self.events
                .iter()
                .find(|event| event.message[0] & 0xF0 == PITCH_BEND)
                .map(|event| {
                    let [lsb, msb] = PITCH_BEND_CENTER;
                    [event.message[0], lsb, msb]
                })
        });

        let at = |seconds: f64, message: [u8; 3]| TimedMidiEvent { seconds, message };
        let mut out: Vec<TimedMidiEvent> = controllers.into_values().map(|m| at(0.0, m)).collect();
        out.extend(bend.map(|m| at(0.0, m)));
        if let Some(message @ [status, pitch, _]) = keyswitch {
            out.push(at(0.0, message));
            out.push(at(release, [0x80 | (status & 0x0F), pitch, 0]));
        }
        out
    }
}

/// KS でない、実際に鳴る note on か。
fn is_played_note_on(event: &TimedMidiEvent) -> bool {
    is_note_on(&event.message) && !is_keyswitch_pitch(event.message[1])
}

fn is_keyswitch_event(event: &TimedMidiEvent) -> bool {
    (is_note_on(&event.message) || is_note_off(&event.message))
        && is_keyswitch_pitch(event.message[1])
}

fn is_controller_or_bend(event: &TimedMidiEvent) -> bool {
    matches!(event.message[0] & 0xF0, CONTROL_CHANGE | PITCH_BEND)
}

#[cfg(test)]
mod tests;
