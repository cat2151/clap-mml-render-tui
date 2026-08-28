//! chord 行から生成される measure セルの表示文字列を組み立てる。
//!
//! 生成対象 track のセルは**手書きが空のまま音が鳴る**（`mml::cell_has_content` 参照）。
//! グリッド上は空セルなので、そのままでは「ここは鳴る」ことも「何が鳴る」ことも
//! 一切見えなかった。chord 行の同じ小節のセルを借りて出す。
//!
//! 借り物であることは呼び出し側が色（紫）で示す。表示だけの責務で、
//! セルの値そのものは書き換えない。

#[cfg(test)]
mod tests;

/// 生成対象セルに出す文字列。生成されないセル / chord 行が空の小節は `None`。
///
/// 桁詰め・切り詰めは行わない（列幅は `super::cell_width` の責務）。
/// 切り詰めは chord 行自身のセルと同じ幅・同じ規則で行われるので、
/// 縦に並べたときに同じ文字列が見える。
pub(super) fn generated_cell_text(
    data: &[Vec<String>],
    track: usize,
    measure: usize,
) -> Option<String> {
    if !crate::mml::cell_is_generated_from_chord_row(data, track, measure) {
        return None;
    }
    let chord = data
        .get(crate::CHORD_TRACK)
        .and_then(|row| row.get(measure))?
        .trim();
    if chord.is_empty() {
        return None;
    }
    Some(chord.to_string())
}
