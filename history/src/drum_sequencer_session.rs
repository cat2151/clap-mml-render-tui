//! Drum Sequencer の永続化用 wire DTO。
//!
//! 入力は kit ごとの pattern ファイル（[`crate::save_drum_pattern_file`]）にあり、ここには
//! 開いていた kit と位置だけを置く。範囲外の pattern 番号・step の扱いは、範囲を知っている
//! 画面側が復元時に決める。

use serde::{Deserialize, Serialize};

/// 選択 kit の参照と、編集していた pattern 番号・カーソル。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrumSequencerSessionState {
    /// catalog の patch 表示名。note 一覧と構成音名は起動のたびに catalog から引き直す。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kit: Option<String>,
    /// 0 始まりの pattern 番号。
    #[serde(default)]
    pub pattern: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_note: Option<u8>,
    /// 0 始まりの step index。
    #[serde(default)]
    pub cursor_step: usize,
}
