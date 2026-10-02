//! keyboard 画面の復帰時情報を、同じ状態で起動する `cmrt kb ...` の 1 行へ書き出す。
//!
//! 文字数を最小にするため、既定値の項目は書かない。buffer 倍率は環境ごとに実現できる値が
//! 違うので書かない。bypass 中の effect 段は音に影響しないので書かない。

use cmrt_effect_chain_select::stage_is_bypassed;
use serde_json::Value;

use crate::session_state::{KeyboardSessionState, NotePlaybackMode};
use crate::state::default_repeat_chords;

/// `state` を再現する `cmrt kb` のコマンド文字列。
pub fn share_command(state: &KeyboardSessionState) -> String {
    let mut command = String::from("cmrt kb");
    let mut push = |flag: &str, value: &str| {
        command.push(' ');
        command.push_str(flag);
        command.push(' ');
        command.push_str(&quote(value));
    };
    if let Some(patch) = state.patch.as_deref().filter(|p| !p.trim().is_empty()) {
        push("-p", patch);
    }
    if let Some(mode) = playback_value(state.note_playback_mode) {
        push("-t", mode);
    }
    match notes_flag(state) {
        NotesFlag::Mml => push("-m", &state.mml),
        NotesFlag::Notes(notes) => push("-n", &notes),
        NotesFlag::None => {}
    }
    for stage in state.effect_chain.iter().filter(|s| !stage_is_bypassed(s)) {
        for (key, value) in stage.as_object().into_iter().flatten() {
            let value = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            push("-e", &format!("{key}={value}"));
        }
    }
    command
}

fn playback_value(mode: NotePlaybackMode) -> Option<&'static str> {
    match mode {
        NotePlaybackMode::Off => None,
        NotePlaybackMode::Auto => Some("auto"),
        NotePlaybackMode::Repeat => Some("repeat"),
        NotePlaybackMode::Arp => Some("arp"),
    }
}

enum NotesFlag {
    Mml,
    Notes(String),
    None,
}

/// 音の書き方。MML から同じ音が起きるなら `-m`、mode の既定と同じなら書かない、それ以外は `-n`。
fn notes_flag(state: &KeyboardSessionState) -> NotesFlag {
    let chords = &state.repeat_chords;
    if !state.mml.is_empty()
        && cmrt_chord::note_progression(&state.mml).is_ok_and(|parsed| &parsed == chords)
    {
        return NotesFlag::Mml;
    }
    let sounding = chords.iter().any(|chord| !chord.is_empty());
    if !sounding || *chords == default_repeat_chords(state.note_playback_mode) {
        return NotesFlag::None;
    }
    NotesFlag::Notes(
        chords
            .iter()
            .map(|chord| {
                chord
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect::<Vec<_>>()
            .join("/"),
    )
}

/// 裸で cmd / PowerShell / bash を通る文字だけなら裸、それ以外は `"..."` で囲む。
fn quote(value: &str) -> String {
    let bare = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._/,=+:-".contains(c));
    if bare {
        value.to_string()
    } else {
        format!("\"{value}\"")
    }
}

#[cfg(test)]
mod tests;
