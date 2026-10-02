//! `cmrt kb` の引数を keyboard 画面の復帰時情報（[`KeyboardSessionState`]）へ起こす。
//!
//! 書かなかった項目は [`KeyboardSessionState::default()`] の値になる。
//! `buffer_multiplier` は引数に無く、保存済みの値へ差し替えるのは起動側の責務。

use clap::{Args, ValueEnum};
use cmrt_tui_core::keyboard_session_state::{KeyboardSessionState, NotePlaybackMode};

#[derive(Debug, Args)]
pub(super) struct KeyboardArgs {
    /// 音色の display 文字列
    #[arg(short = 'p', long = "patch", value_name = "DISPLAY")]
    patch: Option<String>,
    /// note 再生モード（keyboard 画面の `t`）
    #[arg(short = 't', long = "playback", value_name = "MODE", value_enum)]
    playback: Option<PlaybackArg>,
    /// 鳴らす MML（keyboard 画面の `i` で確定するもの）。音はここから起こす
    #[arg(short = 'm', long = "mml", value_name = "MML", value_parser = parse_mml, conflicts_with = "notes")]
    mml: Option<MmlArg>,
    /// 鳴らす note number。`,` で和音、`/` で和音の区切り（例 60,65,67/62,67,71）
    #[arg(short = 'n', long = "notes", value_name = "NOTES", value_parser = parse_notes)]
    notes: Option<NoteChords>,
    /// effect chain の 1 段（KEY=VALUE。KEY は server へ送る JSON key）。段数ぶん繰り返す
    #[arg(short = 'e', long = "effect", value_name = "KEY=VALUE", value_parser = parse_effect)]
    effects: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PlaybackArg {
    Off,
    Auto,
    Repeat,
    Arp,
}

impl From<PlaybackArg> for NotePlaybackMode {
    fn from(arg: PlaybackArg) -> Self {
        match arg {
            PlaybackArg::Off => Self::Off,
            PlaybackArg::Auto => Self::Auto,
            PlaybackArg::Repeat => Self::Repeat,
            PlaybackArg::Arp => Self::Arp,
        }
    }
}

/// 検証済みの MML と、そこから起こした音。
#[derive(Debug, Clone)]
struct MmlArg {
    mml: String,
    chords: Vec<Vec<u8>>,
}

fn parse_mml(value: &str) -> Result<MmlArg, String> {
    let chords = cmrt_chord::note_progression(value)?;
    Ok(MmlArg {
        mml: value.to_string(),
        chords,
    })
}

/// `-n` の値。clap が `Option<Vec<_>>` を「複数回の指定」と読むのを避けるための包み。
#[derive(Debug, Clone)]
struct NoteChords(Vec<Vec<u8>>);

fn parse_notes(value: &str) -> Result<NoteChords, String> {
    value
        .split('/')
        .map(|chord| {
            chord
                .split(',')
                .map(|note| {
                    note.trim()
                        .parse::<u8>()
                        .ok()
                        .filter(|note| *note <= 127)
                        .ok_or_else(|| format!("note number は 0..=127 の整数: `{note}`"))
                })
                .collect()
        })
        .collect::<Result<_, _>>()
        .map(NoteChords)
}

fn parse_effect(value: &str) -> Result<serde_json::Value, String> {
    let (key, stage_value) = value
        .split_once('=')
        .ok_or_else(|| format!("effect は KEY=VALUE の形で指定する: `{value}`"))?;
    let mut stage = serde_json::Map::new();
    stage.insert(
        key.to_string(),
        serde_json::Value::String(stage_value.to_string()),
    );
    Ok(serde_json::Value::Object(stage))
}

impl KeyboardArgs {
    pub(super) fn into_session_state(self) -> KeyboardSessionState {
        let mut state = KeyboardSessionState {
            patch: self.patch,
            note_playback_mode: self.playback.map(Into::into).unwrap_or_default(),
            effect_chain: self.effects,
            ..KeyboardSessionState::default()
        };
        if let Some(MmlArg { mml, chords }) = self.mml {
            state.mml = mml;
            state.repeat_chords = chords;
        } else if let Some(NoteChords(notes)) = self.notes {
            state.repeat_chords = notes;
        }
        state.normalize();
        state
    }
}

#[cfg(test)]
mod tests;
