//! グリッドの行 index → play server の live instance id の対応。
//!
//! `CachePlayer` backend は演奏 track 1 本を live instance 1 本へ割り当てて鳴らす。
//! 音を鳴らさない行（Tempo / chord）は instance を持たないので、行 index と instance id はずれる。
//! 上限があるのは、サーバーの instance 数（`CMRT_LIVE_INSTANCE_COUNT`）が起動時にしか決まらず、
//! DAW が起動後に track を増やしても増えないため。溢れたぶんは鳴らさずログへ落とす。

use cmrt_realtime_play::{InstanceId, DEFAULT_LIVE_INSTANCE_COUNT};

use crate::tracks::FIRST_PLAYABLE_TRACK;

/// live 経路で同時に鳴らせる演奏 track の上限（サーバーの既定 instance 数 = bank 2 本ぶんを除いた、UI が見せるトラック数）。
pub(crate) const MAX_LIVE_TRACKS: usize = DEFAULT_LIVE_INSTANCE_COUNT;

/// グリッドの行 index → live instance id。音を鳴らさない行と上限を超えた行は `None`。
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
