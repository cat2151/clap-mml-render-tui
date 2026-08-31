//! realtime play server（外部プロセス）の監督と、そこへの MIDI / MML 送信。
//!
//! notepad 再生・DAW・keyboard 画面が共有する再生インフラ。`RealtimePlayServerSupervisor`
//! が HTTP リクエストを、[`supervisor_process`] がそのサーバープロセスの生存管理を担い、
//! `fast_midi_ipc` が Windows の共有メモリ経由の低レイテンシ MIDI 送信を担う。

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use anyhow::{anyhow, Result};

use cmrt_runtime::Config;

pub mod fast_midi_ipc;
mod live_ipc;
mod logging;
mod process;
mod startup_failure;
mod supervisor_process;

pub use logging::set_log_sink;

use logging::{log_realtime_play_event, truncate_for_log};

pub use fast_midi_ipc::{
    FastMidiEvent, InstanceId, LimiterMeter, LiveTempoChange, LiveTimelineConfig, TimelineId,
    TimelineMidiEvent, TimingMetrics, INSTANCE_COUNT, MAX_MIDI_MESSAGES,
};
pub use live_ipc::{PatchVoicing, StandbyPatchRequest, VoicingReport, STANDBY_LOAD_TIMEOUT};
pub use startup_failure::ServerStartupFailure;

use supervisor_process::PlayServerState;

/// UI が見せるトラック数の既定値。サーバーが生成する instance 数はこの 2 倍になる。
pub const DEFAULT_LIVE_INSTANCE_COUNT: usize = INSTANCE_COUNT / BANK_COUNT;
/// UI が見せるトラック数（grid sequencer の `t` キーが循環する値）。
///
/// 3 は chord mode 用。行1=chord・行2=bass・行3=4 voice（アルペジオ）で過不足がない。
/// 7 は drum 用。3 に drum 4 role（行4=perc・行5=hi-hat・行6=snare・行7=kick）を足した数。
pub const SUPPORTED_LIVE_INSTANCE_COUNTS: [usize; 7] = [1, 2, 3, 4, 7, 8, 16];
/// サーバーへ要求できる instance 数。トラック数それぞれの bank 2 本ぶん。
///
/// ここを増やすときは play server 側（`realtime-play-server/src/config.rs` の
/// `SUPPORTED_LIVE_INSTANCE_COUNTS`）も必ず揃えること。2 repo に二重定義されている。
pub const SUPPORTED_SERVER_INSTANCE_COUNTS: [usize; 8] = [1, 2, 4, 6, 8, 14, 16, 32];
pub const LIVE_INSTANCE_COUNT_ENV: &str = "CMRT_LIVE_INSTANCE_COUNT";

/// bank の数。grid sequencer の chord mode は、鳴っている bank の裏でもう一方へ
/// 次の patch を先読みし、小節境界で入れ替えることで差し替えの無音をなくす。
pub const BANK_COUNT: usize = 2;

/// live 出力バッファの倍率として指定できる最大値。
///
/// サーバー側のリングは `buffer_size * MAX_LIVE_BUFFER_MULTIPLIER` を起動時に確保するので、
/// ここを広げるときは必ずサーバー側の `MAX_BUFFER_MULTIPLIER` も同じ値へ揃えること。
pub const MAX_LIVE_BUFFER_MULTIPLIER: u16 = 256;

/// 倍率として受け付ける値か（1〜[`MAX_LIVE_BUFFER_MULTIPLIER`] の2冪）。
pub fn is_valid_buffer_multiplier(multiplier: u16) -> bool {
    multiplier.is_power_of_two() && multiplier <= MAX_LIVE_BUFFER_MULTIPLIER
}

/// トラック数に対して、サーバーが生成すべき instance 数（= bank 2 本ぶん）。
pub fn server_instance_count(track_count: usize) -> usize {
    normalize_live_instance_count(track_count) * BANK_COUNT
}

const PLAY_SERVER_PLAY_PATH: &str = "/play";
const PLAY_SERVER_PLAY_MML_PATH: &str = "/play-mml";
const PLAY_SERVER_STOP_PATH: &str = "/stop";
const PLAY_CONTENT_TYPE_MIDI: &str = "audio/midi";
const PLAY_CONTENT_TYPE_MML: &str = "text/plain; charset=utf-8";

pub struct RealtimePlayServerSupervisor {
    port: u16,
    command: String,
    live_instance_count: usize,
    agent: ureq::Agent,
    state: Mutex<PlayServerState>,
    fast_client: Mutex<Option<fast_midi_ipc::FastMidiClient>>,
    /// command の同期応答待ちに塞がれず、出力 callback の drop 数だけを読む handle。
    fast_underrun_reader: Mutex<Option<fast_midi_ipc::FastMidiUnderrunReader>>,
    live_buffer_multiplier: Mutex<u16>,
    startup_progress: Arc<Mutex<Option<RealtimePlayServerStartupProgress>>>,
    next_request_id: AtomicU64,
    /// 直近に server が落ちた理由。`state` とは別の錠にしてあるのは、
    /// UI（描画スレッド）が spawn 待ちにブロックされずに読めるようにするため。
    last_startup_failure: Mutex<Option<ServerStartupFailure>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RealtimePlayServerStartupProgress {
    pub initialized_instances: usize,
    pub total_instances: usize,
}

enum PlayRequestError {
    Server { status: u16, message: String },
    Transport(String),
}

impl RealtimePlayServerSupervisor {
    pub fn new(cfg: &Config) -> Self {
        Self::with_live_instance_count(cfg, server_instance_count(DEFAULT_LIVE_INSTANCE_COUNT))
    }

    /// `live_instance_count` は**サーバーが生成する instance 数**であって、UI が見せる
    /// トラック数ではない。トラック数から求めるには [`server_instance_count`] を使う。
    pub fn with_live_instance_count(cfg: &Config, live_instance_count: usize) -> Self {
        assert!(
            SUPPORTED_SERVER_INSTANCE_COUNTS.contains(&live_instance_count),
            "server instance count must be one of {SUPPORTED_SERVER_INSTANCE_COUNTS:?}"
        );
        // http_status_as_error(false): 4xx/5xx も Ok として受け取り、
        // 本文をエラーメッセージへ載せられるようにする（ureq 3 の StatusCode error は本文を持たない）。
        let agent = ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .timeout_send_body(Some(Duration::from_secs(30)))
                .timeout_recv_response(Some(Duration::from_secs(30)))
                .timeout_recv_body(Some(Duration::from_secs(30)))
                .http_status_as_error(false)
                .build(),
        );
        Self {
            port: cfg.realtime_play_server_port,
            command: cfg.realtime_play_server_command.clone(),
            live_instance_count,
            agent,
            state: Mutex::new(PlayServerState::default()),
            fast_client: Mutex::new(None),
            fast_underrun_reader: Mutex::new(None),
            live_buffer_multiplier: Mutex::new(4),
            startup_progress: Arc::new(Mutex::new(None)),
            next_request_id: AtomicU64::new(1),
            last_startup_failure: Mutex::new(None),
        }
    }

    pub fn play_smf(&self, smf_bytes: Vec<u8>) -> Result<()> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        log_realtime_play_event(format!(
            "request_id={request_id} action=play retry=0 bytes={}",
            smf_bytes.len()
        ));

        let mut retry = 0;
        loop {
            let server_generation = self.ensure_started()?;
            match self.send_play_once(&smf_bytes) {
                Ok(()) => return Ok(()),
                Err(PlayRequestError::Server { message, .. }) => return Err(anyhow!(message)),
                Err(PlayRequestError::Transport(message)) => {
                    retry += 1;
                    log_realtime_play_event(format!(
                        "request_id={request_id} action=play retry={retry} transport_error=\"{}\"",
                        truncate_for_log(&message, 160)
                    ));
                    self.recover_after_transport_failure(server_generation)?;
                }
            }
        }
    }

    /// MML（行頭の音色 JSON 込み）を /play-mml へ送る。
    /// 旧サーバー（/play-mml 未対応）の場合は /play へ SMF でフォールバックする
    /// （音色は反映されないが従来どおり再生される）。
    pub fn play_mml(&self, mml: &str, fallback_smf: Vec<u8>) -> Result<()> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        log_realtime_play_event(format!(
            "request_id={request_id} action=play-mml retry=0 chars={}",
            mml.chars().count()
        ));

        let mut retry = 0;
        loop {
            let server_generation = self.ensure_started()?;
            match self.send_play_mml_once(mml) {
                Ok(()) => return Ok(()),
                Err(PlayRequestError::Server {
                    status: 404 | 405, ..
                }) => {
                    log_realtime_play_event(format!(
                        "request_id={request_id} action=play-mml fallback=play \
                         reason=\"play server does not support /play-mml (update the server)\""
                    ));
                    return self.play_smf(fallback_smf);
                }
                Err(PlayRequestError::Server { message, .. }) => return Err(anyhow!(message)),
                Err(PlayRequestError::Transport(message)) => {
                    retry += 1;
                    log_realtime_play_event(format!(
                        "request_id={request_id} action=play-mml retry={retry} transport_error=\"{}\"",
                        truncate_for_log(&message, 160)
                    ));
                    self.recover_after_transport_failure(server_generation)?;
                }
            }
        }
    }

    pub fn stop(&self) -> Result<()> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let Some(_server_generation) = self.running_server_generation()? else {
            log_realtime_play_event(format!(
                "request_id={request_id} action=stop skipped=no-server"
            ));
            return Ok(());
        };

        match self.send_stop_once() {
            Ok(()) => Ok(()),
            Err(PlayRequestError::Server { message, .. }) => Err(anyhow!(message)),
            Err(PlayRequestError::Transport(message)) => {
                log_realtime_play_event(format!(
                    "request_id={request_id} action=stop transport_error=\"{}\"",
                    truncate_for_log(&message, 160)
                ));
                Ok(())
            }
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn live_instance_count(&self) -> usize {
        self.live_instance_count
    }

    pub fn startup_progress(&self) -> Option<RealtimePlayServerStartupProgress> {
        *self.startup_progress.lock().unwrap()
    }

    fn send_play_once(&self, smf_bytes: &[u8]) -> std::result::Result<(), PlayRequestError> {
        self.send_post_bytes(
            PLAY_SERVER_PLAY_PATH,
            Some((PLAY_CONTENT_TYPE_MIDI, smf_bytes)),
        )
    }

    fn send_play_mml_once(&self, mml: &str) -> std::result::Result<(), PlayRequestError> {
        self.send_post_bytes(
            PLAY_SERVER_PLAY_MML_PATH,
            Some((PLAY_CONTENT_TYPE_MML, mml.as_bytes())),
        )
    }

    fn send_stop_once(&self) -> std::result::Result<(), PlayRequestError> {
        self.send_post_bytes(PLAY_SERVER_STOP_PATH, None)
    }

    fn send_post_bytes(
        &self,
        path: &str,
        body: Option<(&str, &[u8])>,
    ) -> std::result::Result<(), PlayRequestError> {
        let url = format!("http://127.0.0.1:{}{}", self.port, path);
        let request = self.agent.post(&url);
        let response = match body {
            Some((content_type, body)) => request.header("Content-Type", content_type).send(body),
            None => request.send_empty(),
        };
        match response {
            Ok(mut response) => {
                let status = response.status().as_u16();
                if response.status().is_success() {
                    return Ok(());
                }
                let body = response_body(&mut response);
                let body = body.trim();
                let message = if body.is_empty() {
                    format!("realtime play server returned HTTP {status}")
                } else {
                    format!("realtime play server returned HTTP {status}: {body}")
                };
                Err(PlayRequestError::Server { status, message })
            }
            Err(error) => Err(PlayRequestError::Transport(error.to_string())),
        }
    }
}

pub fn normalize_live_instance_count(count: usize) -> usize {
    if SUPPORTED_LIVE_INSTANCE_COUNTS.contains(&count) {
        count
    } else {
        DEFAULT_LIVE_INSTANCE_COUNT
    }
}

pub fn next_live_instance_count(current: usize) -> usize {
    let current = normalize_live_instance_count(current);
    let index = SUPPORTED_LIVE_INSTANCE_COUNTS
        .iter()
        .position(|count| *count == current)
        .expect("normalized live instance count is supported");
    SUPPORTED_LIVE_INSTANCE_COUNTS[(index + 1) % SUPPORTED_LIVE_INSTANCE_COUNTS.len()]
}

fn response_body(response: &mut ureq::http::Response<ureq::Body>) -> String {
    response.body_mut().read_to_string().unwrap_or_default()
}

#[cfg(test)]
mod tests;
