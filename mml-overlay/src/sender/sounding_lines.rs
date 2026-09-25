//! 行の演奏を鳴らしている（release・effect の余韻を含む）instance の記録。
//!
//! 音色を変える行はもう一方の bank の instance で鳴るので、前の行の音はその instance に
//! 残って release する。[`super::MmlOverlaySender::fade_out_line`] はこの記録の instance を
//! fadeout する。**記録は worker と呼び出し側のスレッドで共有する。** fadeout は準備を待つ
//! command の列の後ろに並べず、呼び出し側のスレッドからその場で送るため。

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

#[derive(Clone, Default)]
pub(super) struct SoundingLines(Arc<Mutex<BTreeSet<u8>>>);

impl SoundingLines {
    /// `instance_id` で行を鳴らし始める。timeline を張る前に記録する（張った直後に届いた
    /// fadeout がこの instance を取りこぼさないため）。
    pub(super) fn record(&self, instance_id: u8) {
        self.0.lock().unwrap().insert(instance_id);
    }

    /// 全部の音を server 管理の全NoteOff で止めた。
    pub(super) fn clear(&self) {
        self.0.lock().unwrap().clear();
    }

    /// 記録を空にして、記録していた instance を返す（fadeout した instance は鳴らない）。
    pub(super) fn take(&self) -> Vec<u8> {
        std::mem::take(&mut *self.0.lock().unwrap())
            .into_iter()
            .collect()
    }
}
