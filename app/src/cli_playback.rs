use chord2mml_core::convert as chord_to_mml;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum CliPlaybackMml {
    Chord { chord: String, mml: String },
    Mml(String),
}

impl CliPlaybackMml {
    pub(super) fn mml(&self) -> &str {
        match self {
            Self::Chord { mml, .. } | Self::Mml(mml) => mml,
        }
    }
}

pub(super) fn cli_playback_mml(input: &str) -> CliPlaybackMml {
    match chord_to_mml(input) {
        Ok(mml) => CliPlaybackMml::Chord {
            chord: input.to_string(),
            mml,
        },
        Err(_) => CliPlaybackMml::Mml(input.to_string()),
    }
}

#[cfg(test)]
mod tests;
