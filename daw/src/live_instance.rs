//! グリッドの行 index → play server の live instance id の対応。
//!
//! `CachePlayer` backend は、演奏 track それぞれを play server の live instance
//! 1 本へ割り当てて鳴らす（1 instance = 1 track = そのとき鳴らすキャッシュ WAV 1 本）。
//! 行 index と instance id はずれる。グリッドの行 0 は Tempo、行 1 は chord 行で
//! **どちらも音を鳴らさない**ので、instance を持つのは行 2（[`FIRST_PLAYABLE_TRACK`]）以降だけ。
//!
//! | 行 index | instance |
//! |---|---|
//! | 0（Tempo） | なし |
//! | 1（chord） | なし |
//! | 2 | 0 |
//! | 3 | 1 |
//! | 17 | 15 |
//! | 18 以降 | なし（上限超過） |
//!
//! 上限があるのは、サーバーの instance 数が**起動時にしか決まらない**ため。
//! `CMRT_LIVE_INSTANCE_COUNT` で決まった数は後から増やせないので、DAW が起動後に
//! track を増やしても instance は増えない。溢れたぶんは鳴らさずログへ落とす。

use cmrt_realtime_play::{InstanceId, DEFAULT_LIVE_INSTANCE_COUNT};

use crate::tracks::FIRST_PLAYABLE_TRACK;

/// live 経路で同時に鳴らせる演奏 track の上限。
///
/// サーバーの既定 instance 数（bank 2 本ぶんを除いた、UI が見せるトラック数）に合わせる。
pub(crate) const MAX_LIVE_TRACKS: usize = DEFAULT_LIVE_INSTANCE_COUNT;

/// グリッドの行 index → live instance id。
///
/// 音を鳴らさない行（Tempo / chord）と、instance 数の上限を超えた行は `None`。
pub(crate) fn live_instance_for_grid_row(row: usize) -> Option<InstanceId> {
    let index = row.checked_sub(FIRST_PLAYABLE_TRACK)?;
    if index >= MAX_LIVE_TRACKS {
        return None;
    }
    // MAX_LIVE_TRACKS は InstanceId(u8) の範囲に収まる。
    Some(index as InstanceId)
}

#[cfg(test)]
mod tests;
