//! server が共有メモリへ公開している計測値を、コマンドを送らずに読む。

use super::*;

impl FastMidiClient {
    pub fn limiter_meter(&self) -> LimiterMeter {
        let ring = self.mapping.ring();
        LimiterMeter {
            current_reduction_db: f32::from_bits(ring.limiter_current_bits.load(Ordering::Acquire)),
            peak_reduction_db: f32::from_bits(ring.limiter_peak_bits.swap(0, Ordering::AcqRel)),
        }
    }

    /// instance ごとの auto-trim ゲイン（dB）。サーバーが公開している最新値。
    pub fn auto_gain_db(&self) -> [f32; INSTANCE_COUNT] {
        let ring = self.mapping.ring();
        let mut gains = [0.0; INSTANCE_COUNT];
        for (gain, slot) in gains.iter_mut().zip(ring.auto_gain_db_bits.iter()) {
            *gain = f32::from_bits(slot.load(Ordering::Acquire));
        }
        gains
    }

    pub fn underrun_frames(&self) -> u64 {
        self.mapping.ring().underrun_frames.load(Ordering::Acquire)
    }

    pub fn timing_metrics(&self) -> TimingMetrics {
        let ring = self.mapping.ring();
        loop {
            let before = ring.timing_sequence.load(Ordering::Acquire);
            if before & 1 != 0 {
                std::hint::spin_loop();
                continue;
            }
            let metrics = TimingMetrics {
                events: ring.timing_events.load(Ordering::Relaxed),
                late_events: ring.timing_late_events.load(Ordering::Relaxed),
                late_events_total: ring.timing_late_events_total.load(Ordering::Relaxed),
                max_late_samples: ring.timing_max_late_samples.load(Ordering::Relaxed),
                max_late_us: f64::from_bits(ring.timing_max_late_us_bits.load(Ordering::Relaxed)),
                output_lead_min_frames: ring.timing_output_lead_min_frames.load(Ordering::Relaxed),
                output_lead_max_frames: ring.timing_output_lead_max_frames.load(Ordering::Relaxed),
                process_load_p95: f32::from_bits(
                    ring.timing_process_load_p95_bits.load(Ordering::Relaxed),
                ),
                process_load_max: f32::from_bits(
                    ring.timing_process_load_max_bits.load(Ordering::Relaxed),
                ),
            };
            if ring.timing_sequence.load(Ordering::Acquire) == before {
                return metrics;
            }
        }
    }
}
