//! 再生セッションの世代管理（画面横断で共有）。
//!
//! notepad / keyboard / loop browser のどの画面から再生を始めても、
//! 「古いセッションの音や状態更新が最新の再生を上書きする」レースを防ぐ必要がある。
//! 世代トークン（`session`）・現在の sink・`play_state` を1型へ閉じ込め、
//! 「このセッションがまだ現行か」の判定を必ずここで通す。
//!
//! `Arc` フィールドだけで構成されており `clone()` は共有ハンドルの複製になる。
//! 画面ごとに clone を持たせても、世代・sink・状態は同じものを指す。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cmrt_realtime_play::RealtimePlayServerSupervisor;

use crate::play_state::PlayState;

/// 本アプリのレンダリング結果は常にインターリーブステレオ。
/// rodio 0.22 の `ChannelCount` は `NonZero<u16>` なので、生成箇所ごとに
/// unwrap を書かずに済むよう定数で共有する。
pub const STEREO: rodio::ChannelCount = rodio::ChannelCount::new(2).unwrap();

#[derive(Clone)]
pub struct PlaybackSession {
    play_state: Arc<Mutex<PlayState>>,
    session: Arc<AtomicU64>,
    active_sink: Arc<Mutex<Option<Arc<rodio::Player>>>>,
    /// realtime play server backend のときだけ設定される。世代更新時に発音を止める。
    realtime_play_server: Option<Arc<RealtimePlayServerSupervisor>>,
}

impl PlaybackSession {
    pub fn new(realtime_play_server: Option<Arc<RealtimePlayServerSupervisor>>) -> Self {
        Self {
            play_state: Arc::new(Mutex::new(PlayState::Idle)),
            session: Arc::new(AtomicU64::new(0)),
            active_sink: Arc::new(Mutex::new(None)),
            realtime_play_server,
        }
    }

    pub fn play_state(&self) -> &Arc<Mutex<PlayState>> {
        &self.play_state
    }

    /// 現在の再生状態のスナップショット。描画側はこれを使い、
    /// 同じロック内で複数回状態を読んで不整合を作らないようにする。
    pub fn play_state_snapshot(&self) -> PlayState {
        self.play_state.lock().unwrap().clone()
    }

    pub fn set_play_state(&self, next_state: PlayState) {
        *self.play_state.lock().unwrap() = next_state;
    }

    pub fn session_token(&self) -> &Arc<AtomicU64> {
        &self.session
    }

    pub fn active_sink(&self) -> &Arc<Mutex<Option<Arc<rodio::Player>>>> {
        &self.active_sink
    }

    pub fn realtime_play_server(&self) -> Option<&Arc<RealtimePlayServerSupervisor>> {
        self.realtime_play_server.as_ref()
    }

    /// 新しい再生セッションを開始し、その世代トークンを返す。
    /// 直前のセッションの sink と realtime play server の発音は停止する。
    pub fn begin(&self) -> u64 {
        let session = self.session.fetch_add(1, Ordering::AcqRel) + 1;
        if let Some(sink) = self.active_sink.lock().unwrap().take() {
            sink.stop();
        }
        if let Some(play_server) = &self.realtime_play_server {
            let _ = play_server.stop();
        }
        session
    }

    pub fn is_current(&self, session: u64) -> bool {
        session_is_current(&self.session, session)
    }

    /// `session` が現行の場合だけ再生状態を更新する。
    pub fn set_play_state_if_current(&self, session: u64, next_state: PlayState) {
        set_play_state_for_session(&self.play_state, &self.session, session, next_state);
    }
}

/// 世代トークンが現行かどうか。ワーカースレッドは `Arc` を個別に持つため、
/// `PlaybackSession` を丸ごと渡さずに済むよう自由関数でも提供する。
pub fn session_is_current(session_token: &AtomicU64, session: u64) -> bool {
    session_token.load(Ordering::Acquire) == session
}

pub fn set_play_state_for_session(
    state: &Mutex<PlayState>,
    session_token: &AtomicU64,
    session: u64,
    next_state: PlayState,
) {
    let mut state = state.lock().unwrap();
    if session_is_current(session_token, session) {
        *state = next_state;
    }
}

pub fn clear_active_sink_for_session(
    active_sink: &Mutex<Option<Arc<rodio::Player>>>,
    session_token: &AtomicU64,
    session: u64,
) {
    let mut active_sink = active_sink.lock().unwrap();
    if session_is_current(session_token, session) {
        active_sink.take();
    }
}

pub fn play_samples_for_session(
    state: &Mutex<PlayState>,
    session_token: &AtomicU64,
    active_sink: &Mutex<Option<Arc<rodio::Player>>>,
    session: u64,
    sample_rate: u32,
    samples: Vec<f32>,
    msg: String,
) {
    if !session_is_current(session_token, session) {
        return;
    }

    let sample_rate = match rodio::SampleRate::new(sample_rate) {
        Some(sample_rate) => sample_rate,
        None => {
            clear_active_sink_for_session(active_sink, session_token, session);
            set_play_state_for_session(
                state,
                session_token,
                session,
                PlayState::Err("エラー: sample rate が 0 です".to_string()),
            );
            return;
        }
    };
    // device sink を drop すると再生が止まるため、sleep_until_end まで保持する。
    let device_sink = match crate::audio_output::open_default_sink() {
        Ok(device_sink) => device_sink,
        Err(e) => {
            clear_active_sink_for_session(active_sink, session_token, session);
            set_play_state_for_session(
                state,
                session_token,
                session,
                PlayState::Err(format!("エラー: audio init failed: {e}")),
            );
            return;
        }
    };
    let sink = Arc::new(rodio::Player::connect_new(device_sink.mixer()));
    sink.append(rodio::buffer::SamplesBuffer::new(
        STEREO,
        sample_rate,
        samples,
    ));
    {
        let mut active_sink_guard = active_sink.lock().unwrap();
        if !session_is_current(session_token, session) {
            sink.stop();
            return;
        }
        *active_sink_guard = Some(Arc::clone(&sink));
    }
    sink.sleep_until_end();

    clear_active_sink_for_session(active_sink, session_token, session);
    set_play_state_for_session(state, session_token, session, PlayState::Done(msg));
}
