//! グリッドの行（track）の役割と、保存ファイル上の track 番号との対応。
//!
//! グリッドの行は 3 種類ある。
//!
//! | 行 index | 役割 |
//! |---|---|
//! | 0 | Tempo/conductor（拍子 JSON + テンポ。全 track の先頭へ前置される） |
//! | 1 | chord 行（コード進行を書く専用行。**音は鳴らさない**） |
//! | 2.. | 演奏 track |
//!
//! chord 行は行 index 1 に**割り込んで**入ったため、行 index と
//! 「保存ファイル上の track 番号 / 画面に出す track 番号」がずれる。
//! そのずれをこのモジュール 1 か所に閉じる。
//!
//! 保存ファイル（session save / project file）の track 番号は
//! **chord 行が入る前の番号のまま**にしてある（0 = Tempo, 1.. = 演奏 track）。
//! chord 行はファイル上では別のフィールドに置く。こうすると
//!
//! - chord 行を使っていない既存セーブを読んで保存し直してもファイルが 1 バイトも変わらない
//! - 画面の `T1` と、保存ファイルの `track: 1` が同じものを指し続ける
//!
//! の 2 つが同時に満たせる。

/// Tempo/conductor 行。
pub(crate) const TEMPO_TRACK: usize = 0;
/// chord 行。コード進行を書く専用行で、この行自体はレンダリングされない。
pub(crate) const CHORD_TRACK: usize = 1;
/// 演奏 track の先頭行。
pub(crate) const FIRST_PLAYABLE_TRACK: usize = 2;
/// 保存ファイル上で最初の演奏 track に付く番号。
pub(crate) const FIRST_SAVED_PLAYABLE_TRACK: usize = 1;

/// 保存ファイル上の track 番号 → グリッドの行 index。
pub(crate) fn grid_row_from_saved_track(saved_track: usize) -> usize {
    if saved_track == TEMPO_TRACK {
        TEMPO_TRACK
    } else {
        saved_track + (FIRST_PLAYABLE_TRACK - FIRST_SAVED_PLAYABLE_TRACK)
    }
}

/// グリッドの行 index → 保存ファイル上の track 番号。
///
/// chord 行はファイル上の track 一覧には含めない（専用フィールドへ置く）ため `None`。
pub(crate) fn saved_track_from_grid_row(row: usize) -> Option<usize> {
    match row {
        TEMPO_TRACK => Some(TEMPO_TRACK),
        CHORD_TRACK => None,
        row => Some(row - (FIRST_PLAYABLE_TRACK - FIRST_SAVED_PLAYABLE_TRACK)),
    }
}

/// 保存ファイル上の track 数（chord 行を含まない）→ グリッドの行数。
pub(crate) fn grid_track_count_from_saved(saved_track_count: usize) -> usize {
    saved_track_count + 1
}

/// グリッドの行数 → 保存ファイル上の track 数（chord 行を含まない）。
pub(crate) fn saved_track_count_from_grid(grid_track_count: usize) -> usize {
    grid_track_count.saturating_sub(1)
}

/// ログや mixer overlay に出す track 番号。
///
/// **保存ファイル上の番号（= 画面の `T1` の番号）に合わせる。**
/// 行 index をそのまま出すと chord 行のぶんずれて、画面の `T1` とログの `track1` が
/// 別の行を指してしまう。chord 行はレンダリングもされないので番号を持たない
/// （見出し用に行 index をそのまま返す）。
pub(crate) fn track_display_number(row: usize) -> usize {
    saved_track_from_grid_row(row).unwrap_or(row)
}

/// その行がオーディオとしてレンダリングされるか。
///
/// chord 行の中身は MML ではなくコード進行なので、レンダリングにかけてはいけない。
pub(crate) fn track_renders_audio(row: usize) -> bool {
    row != CHORD_TRACK
}

/// 行頭に出す track ラベル。
///
/// 演奏 track の番号は**保存ファイル上の番号と同じ**にする（chord 行のぶんずらさない）。
pub(crate) fn track_label(row: usize) -> String {
    match row {
        TEMPO_TRACK => "Tempo".to_string(),
        CHORD_TRACK => "Chord".to_string(),
        row => format!("T{}", saved_track_from_grid_row(row).unwrap_or(row)),
    }
}

#[cfg(test)]
mod tests;
