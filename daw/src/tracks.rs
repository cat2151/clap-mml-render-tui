//! グリッドの行（track）の役割と、保存ファイル上の track 番号との対応。
//!
//! chord 行は行 index 1 に割り込んで入ったので、行 index と「保存ファイル上 / 画面に出す
//! track 番号」がずれる。そのずれはこのモジュール 1 か所に閉じる。
//!
//! 保存ファイル（session save / project file）の track 番号は chord 行が入る前のまま
//! （0 = Tempo, 1.. = 演奏 track）で、chord 行は別のフィールドに置く。既存セーブを読んで
//! 保存し直しても 1 バイトも変わらず、画面の `T1` と保存ファイルの `track: 1` が同じものを指し続ける。

/// Tempo/conductor 行（拍子 JSON + テンポ。全 track の先頭へ前置される）。
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

/// グリッドの行 index → 保存ファイル上の track 番号。chord 行は専用フィールドへ置くので `None`。
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

/// ログや mixer overlay に出す track 番号。保存ファイル上の番号（= 画面の `T1`）に合わせる。
/// 行 index をそのまま出すと chord 行のぶんずれて、画面の `T1` とログの `track1` が別の行を指す。
pub(crate) fn track_display_number(row: usize) -> usize {
    saved_track_from_grid_row(row).unwrap_or(row)
}

/// その行がオーディオとしてレンダリングされるか。chord 行の中身はコード進行で MML ではない。
pub(crate) fn track_renders_audio(row: usize) -> bool {
    row != CHORD_TRACK
}

/// 行頭に出す track ラベル。演奏 track の番号は保存ファイル上の番号と同じ。
pub(crate) fn track_label(row: usize) -> String {
    match row {
        TEMPO_TRACK => "Tempo".to_string(),
        CHORD_TRACK => "Chord".to_string(),
        row => format!("T{}", saved_track_from_grid_row(row).unwrap_or(row)),
    }
}

#[cfg(test)]
mod tests;
