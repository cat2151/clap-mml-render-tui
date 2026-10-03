//! サーバーが待ち行列満杯で捨てたイベントの見張り。
//!
//! サーバーは捨てた数の累計を共有メモリへ出すだけで、どのイベントを捨てたかは返さない。
//! 捨てた中に note off があれば音が鳴り残るので、timeline を張る直前の累計を控えておき、
//! 次の停止の前に増えていたら、その停止を全音停止にする。

use super::super::sink::SoundSink;

#[derive(Default)]
pub(super) struct ServerDrops {
    /// timeline を張る直前の累計。張った演奏が無ければ `None`。
    baseline: Option<u64>,
}

impl ServerDrops {
    /// これから張る timeline の分を数え始める。
    pub(super) fn watch(&mut self, sink: &impl SoundSink) {
        self.baseline = Some(sink.dropped_events_total());
    }

    /// 控えた時点から増えた数。増えていなければ `None`。見張りはここで終わる。
    ///
    /// 累計が控えより小さいのはサーバーが起動し直したときで、増えたとはみなさない。
    pub(super) fn take_increase(&mut self, sink: &impl SoundSink) -> Option<u64> {
        let baseline = self.baseline.take()?;
        let now = sink.dropped_events_total();
        (now > baseline).then(|| now - baseline)
    }
}
