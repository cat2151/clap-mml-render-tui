//! `chord_chart.json` の保存と復元。
//!
//! パスは `cmrt-history` が決め（`chord_chart_file_path`）、serde はこちらが持つ。
//! `daw.json` と同じ役割分担。
//!
//! 読み込みは **絶対に panic させない**。ファイルが壊れていても、
//! 参照先の無い arrangement が入っていても、必ず何らかの答えを返す。
//! そのために保存形式は [`Song`] を直接 derive で読むのではなく、
//! 専用の DTO（[`SongFile`]）を通す。[`Song`] 側の `next_section_id` のような
//! 「保存しない状態」と、重複 id / 幽霊参照の掃除をここへ閉じ込めるため。
//!
//! **読めなかったときに曲をでっち上げない。** [`load_song`] は `None` を返すだけで、
//! そこから何を始めるかは画面側（[`crate::ChordChartScreen::enter`] の自動抽選）が決める。
//! ここで既定の曲を作ると、コード進行をこの crate へハードコードすることになる。

use serde::{Deserialize, Serialize};

use crate::song::{Section, SectionId, Song};

/// `chord_chart.json` のルート。
///
/// バージョン番号は持たない。未知のフィールドは serde が無視し、欠けたフィールドは
/// 既定値へ丸まるので、番号で分岐しなくても古い / 新しいファイルで壊れない
/// （番号を見ないのに書き出すのは、読む側に無い分岐を装う嘘になる）。
#[derive(Debug, Deserialize, Serialize)]
struct SongFile {
    /// chord2mml の指定をそのまま入れた 1 本の文字列（`"Key=C BPM120"`）。
    /// 欠けているとき（= Key / BPM を別々に持っていた旧形式）は既定値へ落とす。
    #[serde(default = "default_prefix")]
    prefix: String,
    #[serde(default)]
    sections: Vec<SectionFile>,
    /// [`SectionId`] の裸の数値の並び（`"arrangement": [1, 1, 2]`）。
    #[serde(default)]
    arrangement: Vec<u32>,
}

/// 保存形式の section 1 件。
#[derive(Debug, Deserialize, Serialize)]
struct SectionFile {
    id: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    degrees: String,
}

fn default_prefix() -> String {
    crate::song::DEFAULT_PREFIX.to_string()
}

/// 保存済みの曲を読む。**読めなければ `None`**（ファイルが無い / 壊れている / 保存先が
/// 決まらない）。
///
/// 「section が 0 個で保存された曲」は `Some(空の曲)` として返す。`None` と混ぜると、
/// 全部消してから再起動したときに消したはずの section が抽選で復活する。
pub fn load_song() -> Option<Song> {
    let path = cmrt_history::chord_chart_file_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let file = serde_json::from_str::<SongFile>(&content).ok()?;
    Some(song_from_file(file))
}

/// 曲を保存する。呼び出し側は失敗をログ 1 行に留め、画面は落とさないこと。
pub fn save_song(song: &Song) -> anyhow::Result<()> {
    let path = cmrt_history::chord_chart_file_path()
        .ok_or_else(|| anyhow::anyhow!("chord_chart.json の保存先を決められない"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(&song_to_file(song))?;
    std::fs::write(&path, json)?;
    Ok(())
}

fn song_to_file(song: &Song) -> SongFile {
    SongFile {
        prefix: song.prefix.clone(),
        sections: song
            .sections
            .iter()
            .map(|section| SectionFile {
                id: section.id.get(),
                name: section.name.clone(),
                degrees: section.degrees.clone(),
            })
            .collect(),
        arrangement: song.arrangement.iter().map(|id| id.get()).collect(),
    }
}

/// DTO を [`Song`] へ落とす。ここが読み込み時の防御の全部。
fn song_from_file(file: SongFile) -> Song {
    let mut song = Song::empty();
    // prefix は**検証しない**（空文字でもそのまま持つ）。読めたものをそのまま返すのが、
    // 「打った文字列が黙って書き換わらない」という `b` の約束。
    song.prefix = file.prefix;
    for section in file.sections {
        let id = SectionId::new(section.id);
        // 同じ id が 2 度出てきたら後のほうを捨てる。残すと arrangement の参照先が
        // どちらか分からなくなる。
        if song.section(id).is_some() {
            continue;
        }
        song.sections.push(Section {
            id,
            name: section.name,
            degrees: section.degrees,
        });
    }
    // 参照先の無い id は黙って捨てる（残すと参照先の引けない幽霊行が画面に出る）。
    song.arrangement = file
        .arrangement
        .into_iter()
        .map(SectionId::new)
        .filter(|id| song.section(*id).is_some())
        .collect();
    // これを呼ばないと load 後の追加が既存 id と衝突する（`next_section_id` は serde(skip)）。
    song.reseed_section_ids();
    song
}

#[cfg(test)]
mod tests;
