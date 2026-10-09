//! Drum Sequencer の pattern ファイルの置き場と読み書き。
//!
//! kit ごとのディレクトリに、pattern 番号ごとの `pattern_NN.mid` を置く。中身（SMF）の
//! 解釈は `cmrt-drum-sequencer` が持ち、この crate はバイト列だけを扱う。空の pattern は
//! ファイルを置かない。

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const DIR_NAME: &str = "drum_patterns";
const FILE_PREFIX: &str = "pattern_";
const FILE_EXTENSION: &str = "mid";

/// kit の pattern ファイルを `(pattern 番号, バイト列)` で返す。番号の範囲は見ない。
/// 保存したことの無い kit は空。
pub fn load_drum_pattern_files(kit: &str) -> Result<Vec<(usize, Vec<u8>)>> {
    let dir = kit_dir(kit)?;
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("pattern の保存先を読めません: {}", dir.display()))
        }
    };
    let mut files = Vec::new();
    for entry in entries {
        let path = entry
            .with_context(|| format!("pattern の保存先を読めません: {}", dir.display()))?
            .path();
        let Some(index) = pattern_index(&path) else {
            continue;
        };
        let bytes = std::fs::read(&path)
            .with_context(|| format!("pattern を読めません: {}", path.display()))?;
        files.push((index, bytes));
    }
    files.sort_by_key(|(index, _)| *index);
    Ok(files)
}

/// `smf` が None なら、その pattern のファイルを消す（無ければ何もしない）。
pub fn save_drum_pattern_file(kit: &str, index: usize, smf: Option<&[u8]>) -> Result<()> {
    let dir = kit_dir(kit)?;
    let path = dir.join(format!("{FILE_PREFIX}{index:02}.{FILE_EXTENSION}"));
    let Some(smf) = smf else {
        return match std::fs::remove_file(&path) {
            Err(error) if error.kind() != ErrorKind::NotFound => {
                Err(error).with_context(|| format!("pattern を消せません: {}", path.display()))
            }
            _ => Ok(()),
        };
    };
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("pattern の保存先を作成できません: {}", dir.display()))?;
    std::fs::write(&path, smf)
        .with_context(|| format!("pattern を書き込めません: {}", path.display()))
}

fn kit_dir(kit: &str) -> Result<PathBuf> {
    let history = super::paths::history_dir().context("pattern の保存先を取得できません")?;
    Ok(history.join(DIR_NAME).join(dir_name(kit)))
}

/// patch 名（`/` 区切りの相対パス）を 1 階層のディレクトリ名にする。ファイル名に使えない文字は
/// `_` に替える。kit の判別は SMF の track 名に残る。
fn dir_name(kit: &str) -> String {
    let name: String = kit
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect();
    // Windows は末尾の `.` と空白を落とすので、別の kit と同じ名前にならないよう残さない。
    let name = name.trim_end_matches(['.', ' ']);
    if name.is_empty() {
        "_".to_string()
    } else {
        name.to_string()
    }
}

fn pattern_index(path: &Path) -> Option<usize> {
    if path.extension()? != FILE_EXTENSION {
        return None;
    }
    path.file_stem()?
        .to_str()?
        .strip_prefix(FILE_PREFIX)?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests;
