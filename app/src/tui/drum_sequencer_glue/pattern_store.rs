//! kit の pattern ファイルと画面の入力の受け渡し。失敗は画面の status に出す文言で返す。

use cmrt_drum_sequencer::{pattern_from_smf, pattern_to_smf, DrumPattern, DrumSequencerScreen};

use crate::history::{load_drum_pattern_files, save_drum_pattern_file};

/// 読めた pattern と、読めなかったものがあればその理由。読めないファイルは空の pattern として扱う。
pub(in crate::tui) fn load_kit_patterns(kit: &str) -> (Vec<(usize, DrumPattern)>, Option<String>) {
    let files = match load_drum_pattern_files(kit) {
        Ok(files) => files,
        Err(error) => return (Vec::new(), Some(format!("pattern を読めません: {error:#}"))),
    };
    let mut patterns = Vec::new();
    let mut failures = Vec::new();
    for (index, bytes) in files {
        match pattern_from_smf(&bytes) {
            Ok(pattern) => patterns.push((index, pattern)),
            Err(error) => failures.push(format!("pattern {index}: {error}")),
        }
    }
    let error =
        (!failures.is_empty()).then(|| format!("読めない pattern: {}", failures.join(", ")));
    (patterns, error)
}

/// 編集のあった pattern を kit のファイルへ書く。空になった pattern はファイルを消す。
pub(super) fn save_edited_pattern(screen: &mut DrumSequencerScreen) -> Result<(), String> {
    let Some(index) = screen.take_edited_pattern() else {
        return Ok(());
    };
    // 編集は kit の行がある間しか起きない。
    let (Some(kit), Some(pattern)) = (screen.kit_name(), screen.pattern_at(index)) else {
        return Ok(());
    };
    let smf = (!pattern.is_empty()).then(|| pattern_to_smf(kit, pattern));
    save_drum_pattern_file(kit, index, smf.as_deref())
        .map_err(|error| format!("pattern を保存できません: {error:#}"))
}

#[cfg(test)]
mod tests;
