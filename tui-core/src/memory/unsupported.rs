//! Windows 以外での計測スタブ。
//!
//! 行数と表示幅は Windows と同じまま「取得不可」を出す（`format` 側が担当）。

use super::MemorySnapshot;

pub(super) fn measure() -> Option<MemorySnapshot> {
    None
}
