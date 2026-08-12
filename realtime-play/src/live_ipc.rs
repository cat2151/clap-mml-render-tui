use std::time::Instant;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use super::{
    fast_midi_ipc::{
        FastIpcError, FastMidiClient, FastMidiEvent, InstanceId, LimiterMeter, LiveTempoChange,
        LiveTimelineConfig, TimelineMidiEvent, TimingMetrics, INSTANCE_COUNT,
    },
    logging::log_realtime_play_event,
    RealtimePlayServerSupervisor,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PatchVoicing {
    Mono,
    Poly,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ProbeReport {
    pub result: PatchVoicing,
    pub ended_note_ids: Vec<u32>,
    pub blocks: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct VoiceInfoReport {
    pub voice_count: u32,
    pub voice_capacity: u32,
    pub supports_overlapping_notes: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SurgeParamsReport {
    pub scene_mode: String,
    pub active_scene: String,
    pub scene_a_play_mode: String,
    pub scene_b_play_mode: String,
    pub result: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct VoicingReport {
    pub decision: PatchVoicing,
    pub probe: ProbeReport,
    pub voice_info: Option<VoiceInfoReport>,
    pub surge: Option<SurgeParamsReport>,
    pub disagreement: bool,
}

/// dB を振幅倍率へ直す（0 dB = 等倍、+6 dB ≒ 2倍）。
pub fn amplitude_from_db(gain_db: f32) -> f32 {
    10.0f32.powf(gain_db / 20.0)
}

impl RealtimePlayServerSupervisor {
    pub fn send_live_events(&self, events: &[FastMidiEvent]) -> Result<LimiterMeter> {
        self.with_fast_client(|client| {
            client.send_events(events)?;
            Ok(client.limiter_meter())
        })
    }

    pub fn begin_live_timeline(&self, config: LiveTimelineConfig) -> Result<()> {
        self.with_fast_client(|client| client.begin_live_timeline(config))
    }

    /// 走っている live timeline の tempo map へテンポ変化点を積む。
    ///
    /// `begin_live_timeline` と違い、サーバー側は timeline を作り直さない
    /// （プラグインの状態もサンプルクロックの原点も動かない）ので、演奏は途切れない。
    pub fn set_live_tempo(&self, change: LiveTempoChange) -> Result<()> {
        self.with_fast_client(|client| client.set_live_tempo(change))
    }

    pub fn send_timeline_events(&self, events: &[TimelineMidiEvent]) -> Result<LimiterMeter> {
        self.with_fast_client(|client| {
            client.send_timeline_events(events)?;
            Ok(client.limiter_meter())
        })
    }

    pub fn send_midi(&self, instance_id: InstanceId, messages: &[[u8; 3]]) -> Result<LimiterMeter> {
        let events = messages
            .iter()
            .map(|message| FastMidiEvent {
                instance_id,
                offset_frames: 0,
                message: *message,
            })
            .collect::<Vec<_>>();
        self.send_live_events(&events)
    }

    pub fn prepare_live_patch(&self, instance_id: InstanceId, patch: Option<&str>) -> Result<()> {
        let started = Instant::now();
        log_realtime_play_event(format!(
            "action=shm-patch-prepare event=start instance={instance_id} patch={patch:?}"
        ));
        let result = self.with_fast_client(|client| client.prepare_patch(instance_id, patch));
        let elapsed_ms = started.elapsed().as_millis();
        match &result {
            Ok(()) => log_realtime_play_event(format!(
                "action=shm-patch-prepare event=success instance={instance_id} \
                 elapsed_ms={elapsed_ms} patch={patch:?}"
            )),
            Err(error) => log_realtime_play_event(format!(
                "action=shm-patch-prepare event=error instance={instance_id} \
                 elapsed_ms={elapsed_ms} patch={patch:?} error=\"{}\"",
                super::logging::truncate_for_log(&format!("{error:#}"), 1_000)
            )),
        }
        result
    }

    pub fn prepare_live_patch_with_voicing(
        &self,
        instance_id: InstanceId,
        patch: Option<&str>,
    ) -> Result<Option<VoicingReport>> {
        self.prepare_live_patch_with_voicing_traced(instance_id, patch, None)
    }

    pub fn prepare_live_patch_with_voicing_traced(
        &self,
        instance_id: InstanceId,
        patch: Option<&str>,
        probe_id: Option<u64>,
    ) -> Result<Option<VoicingReport>> {
        let started = Instant::now();
        let bytes = self.with_fast_client(|client| client.probe_patch(instance_id, patch))?;
        let report = serde_json::from_slice(&bytes)?;
        log_realtime_play_event(format!(
            "action=shm-patch-probe probe_id={} instance={} elapsed_ms={} patch={patch:?}",
            probe_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "untracked".to_string()),
            instance_id,
            started.elapsed().as_millis()
        ));
        Ok(Some(report))
    }

    pub fn stop_live_instance(&self, instance_id: InstanceId) -> Result<()> {
        self.with_fast_client(|client| client.stop(instance_id))
    }

    pub fn stop_live_all(&self) -> Result<()> {
        let mut client = self.fast_client.lock().unwrap();
        match client.as_mut() {
            Some(client) => client.stop_all().map_err(Into::into),
            None => Ok(()),
        }
    }

    pub fn set_live_buffer_multiplier(&self, multiplier: u16) -> Result<()> {
        validate_buffer_multiplier(multiplier)?;
        *self.live_buffer_multiplier.lock().unwrap() = multiplier;
        self.with_fast_client(|client| client.set_buffer_multiplier(multiplier))
    }

    /// live mix で instance へ掛けるゲインを dB で設定する（0 dB が等倍）。
    ///
    /// サーバー側が保持するので、patch 差し替えで live を作り直しても残る。
    /// この機能を持たない古いサーバーはコマンドを拒否するが、その場合も
    /// 「音量差が付かないだけ」で再生は続く。
    pub fn set_live_instance_gain_db(&self, instance_id: InstanceId, gain_db: f32) -> Result<()> {
        let gain = amplitude_from_db(gain_db);
        let result = self.with_fast_client(|client| client.set_instance_gain(instance_id, gain));
        if let Err(error) = &result {
            log_realtime_play_event(format!(
                "action=shm-instance-gain event=error instance={instance_id} \
                 gain_db={gain_db} error=\"{}\"",
                super::logging::truncate_for_log(&format!("{error:#}"), 1_000)
            ));
        }
        result
    }

    pub fn set_connected_live_buffer_multiplier(&self, multiplier: u16) -> Result<()> {
        validate_buffer_multiplier(multiplier)?;
        *self.live_buffer_multiplier.lock().unwrap() = multiplier;
        let mut client = self.fast_client.lock().unwrap();
        match client.as_mut() {
            Some(client) => client.set_buffer_multiplier(multiplier).map_err(Into::into),
            None => Ok(()),
        }
    }

    pub fn remember_live_buffer_multiplier(&self, multiplier: u16) -> Result<()> {
        validate_buffer_multiplier(multiplier)?;
        *self.live_buffer_multiplier.lock().unwrap() = multiplier;
        Ok(())
    }

    pub fn limiter_meter(&self) -> LimiterMeter {
        self.fast_client
            .lock()
            .unwrap()
            .as_ref()
            .map(FastMidiClient::limiter_meter)
            .unwrap_or_default()
    }

    pub fn underrun_frames(&self) -> u64 {
        self.fast_client
            .lock()
            .unwrap()
            .as_ref()
            .map(FastMidiClient::underrun_frames)
            .unwrap_or(0)
    }

    pub fn timing_metrics(&self) -> TimingMetrics {
        self.fast_client
            .lock()
            .unwrap()
            .as_ref()
            .map(FastMidiClient::timing_metrics)
            .unwrap_or_default()
    }

    /// instance ごとに auto-trim が掛けているゲイン（dB）。
    ///
    /// auto gain の判断はサーバー内で完結するので、UI へ「効いているか」を出すには
    /// この読み出しだけが手がかりになる。未接続なら全 0 dB。
    pub fn live_auto_gain_db(&self) -> [f32; INSTANCE_COUNT] {
        self.fast_client
            .lock()
            .unwrap()
            .as_ref()
            .map(FastMidiClient::auto_gain_db)
            .unwrap_or([0.0; INSTANCE_COUNT])
    }

    fn with_fast_client<T>(
        &self,
        operation: impl FnOnce(&mut FastMidiClient) -> Result<T, FastIpcError>,
    ) -> Result<T> {
        self.ensure_started_for_fast_midi()?;
        let mut client = self.fast_client.lock().unwrap();
        if client.is_none() {
            let mut connected = connect_fast_client(self.port)?;
            connected.set_buffer_multiplier(*self.live_buffer_multiplier.lock().unwrap())?;
            *client = Some(connected);
        }
        let result = operation(client.as_mut().expect("client was initialized"));
        if matches!(
            result,
            Err(FastIpcError::ServerStopped | FastIpcError::ProtocolMismatch)
        ) {
            *client = None;
        }
        result.map_err(|error| anyhow!(error))
    }

    /// live mixのinstance別RMS auto-trimを切り替える。
    pub fn set_live_auto_gain_enabled(&self, enabled: bool) -> Result<()> {
        let result = self.with_fast_client(|client| client.set_auto_gain_enabled(enabled));
        if let Err(error) = &result {
            log_realtime_play_event(format!(
                "action=shm-auto-gain event=error enabled={enabled} error=\"{}\"",
                super::logging::truncate_for_log(&format!("{error:#}"), 1_000)
            ));
        }
        result
    }
}

fn validate_buffer_multiplier(multiplier: u16) -> Result<()> {
    if !super::is_valid_buffer_multiplier(multiplier) {
        anyhow::bail!(
            "buffer multiplier must be a power of two up to {}",
            super::MAX_LIVE_BUFFER_MULTIPLIER
        );
    }
    Ok(())
}

#[cfg(windows)]
fn connect_fast_client(port: u16) -> Result<FastMidiClient, FastIpcError> {
    use std::time::Duration;

    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        match FastMidiClient::connect(port) {
            Ok(client) => return Ok(client),
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(not(windows))]
fn connect_fast_client(port: u16) -> Result<FastMidiClient, FastIpcError> {
    FastMidiClient::connect(port)
}

#[cfg(test)]
mod live_gain_tests {
    use super::amplitude_from_db;

    #[test]
    fn zero_db_is_unity_gain() {
        assert!((amplitude_from_db(0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn plus_six_db_roughly_doubles_the_amplitude() {
        let gain = amplitude_from_db(6.0);
        assert!((gain - 1.9953).abs() < 1e-3, "gain={gain}");
    }

    #[test]
    fn minus_six_db_roughly_halves_the_amplitude() {
        let gain = amplitude_from_db(-6.0);
        assert!((gain - 0.5012).abs() < 1e-3, "gain={gain}");
    }

    /// 千分率へ丸めても dB のずれが無視できること（SHM が u32 milli で運ぶため）。
    #[test]
    fn rounding_to_milli_units_keeps_the_db_within_a_hundredth() {
        let gain = amplitude_from_db(6.0);
        let rounded = (gain * 1000.0).round() / 1000.0;
        let db = 20.0 * rounded.log10();
        assert!((db - 6.0).abs() < 0.01, "db={db}");
    }
}
