//! mixer の音量 / mute / solo を、live mix の instance gain へその場で配線する。
//!
//! `CachePlayer` backend では gain が**サーバー側の mix 直前**に掛かるので、
//! ここから送った値はほぼ次のオーディオブロックで効く。
//! rodio 経路（`InProcess`）が「チャンクを append する瞬間に振幅を焼き込む」ために
//! 実測 2.4 秒（＝ 1 小節）遅れていたのに対する、対策そのもの。
//!
//! そのため **`playback_track_gains()`（振幅へ直して焼き込む経路）は live では通さない。**
//! あちらは rodio 経路と、キャッシュ済み WAV を混ぜる preview のためのもの。
//!
//! ## 送るのは「変わったぶんだけ」
//!
//! 直前に送った値を [`crate::playback_runtime::DawPlaybackRuntime::live_track_gains`] に
//! 覚えておき、差分だけ送る。理由は 2 つある。
//!
//! - 音量キー 1 回で 1 コマンドしか飛ばない（ログが読める）
//! - mixer 以外の編集（セル入力など）でも同じ同期関数を通るので、
//!   差分が空なら IPC を 1 バイトも起こさずに帰れる

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use cmrt_realtime_play::{InstanceId, RealtimePlayServerSupervisor};
use cmrt_runtime::RealtimeAudioBackend;

use crate::{
    live_instance::live_instance_for_grid_row, DawApp, DawPlayState, FIRST_PLAYABLE_TRACK,
};

/// 鳴らさない track へ送る「無音相当」の dB。
///
/// live mix の gain は掛け算なので、mute を表す専用の値はサーバーに無い。
/// -120 dB（振幅 1e-6）は 32bit float の音声では聞こえないし、
/// 0.0 と違って「gain を送り忘れた」状態と区別できる。
pub(crate) const SILENT_TRACK_GAIN_DB: f32 = -120.0;

/// live mix の instance 1 本へ送る gain。
///
/// `gain_db` は mixer の整数 dB か [`SILENT_TRACK_GAIN_DB`] のどちらかしか取らないので、
/// 差分判定を `==` でやってよい（計算で作った端数は入らない）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LiveTrackGain {
    pub(crate) row: usize,
    pub(crate) instance: InstanceId,
    pub(crate) gain_db: f32,
}

/// mixer の現在値を、live mix へ送る形へ直す。
///
/// `volume_db` は mixer が持っている dB、`is_audible` は solo / mute を織り込んだ可聴判定。
/// 実サーバーも `DawApp` も無しで単体テストできるよう、どちらも関数で受ける。
pub(crate) fn live_track_gains(
    tracks: usize,
    volume_db: impl Fn(usize) -> i32,
    is_audible: impl Fn(usize) -> bool,
) -> Vec<LiveTrackGain> {
    (FIRST_PLAYABLE_TRACK..tracks)
        .filter_map(|row| {
            let instance = live_instance_for_grid_row(row)?;
            let gain_db = if is_audible(row) {
                volume_db(row) as f32
            } else {
                SILENT_TRACK_GAIN_DB
            };
            Some(LiveTrackGain {
                row,
                instance,
                gain_db,
            })
        })
        .collect()
}

/// 直前に送った値と比べて、実際に送り直す必要があるものだけを返す。
///
/// 送った記録が無い instance は「サーバー側の値が分からない」ので必ず送る。
pub(crate) fn changed_live_track_gains(
    sent: &[LiveTrackGain],
    desired: &[LiveTrackGain],
) -> Vec<LiveTrackGain> {
    // 行と instance の対応は固定なので、まるごと一致していれば「同じ値を送り済み」。
    desired
        .iter()
        .filter(|target| !sent.contains(target))
        .copied()
        .collect()
}

/// 送った gain を 1 行にまとめる。
///
/// 例: `live-gain: row2/i0=-3dB row3/i1=mute`
pub(crate) fn format_live_gain_log(gains: &[LiveTrackGain]) -> String {
    let body = gains
        .iter()
        .map(|gain| {
            if gain.gain_db == SILENT_TRACK_GAIN_DB {
                format!("row{}/i{}=mute", gain.row, gain.instance)
            } else {
                format!("row{}/i{}={}dB", gain.row, gain.instance, gain.gain_db)
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    if body.is_empty() {
        "live-gain: -".to_string()
    } else {
        format!("live-gain: {body}")
    }
}

/// gain を実サーバーへ送る。失敗した行だけログへ落とす。
pub(crate) fn send_live_track_gains(
    play_server: &RealtimePlayServerSupervisor,
    gains: &[LiveTrackGain],
    log_lines: &Arc<Mutex<VecDeque<String>>>,
) {
    for gain in gains {
        if let Err(error) = play_server.set_live_instance_gain_db(gain.instance, gain.gain_db) {
            crate::append_log_line(
                log_lines,
                format!(
                    "live-gain: send failed row={} instance={} gain_db={} error=\"{error:#}\"",
                    gain.row, gain.instance, gain.gain_db
                ),
            );
        }
    }
    crate::append_log_line(log_lines, format_live_gain_log(gains));
}

impl DawApp {
    /// いまの mixer が live mix へ望む gain。
    pub(crate) fn desired_live_track_gains(&self) -> Vec<LiveTrackGain> {
        live_track_gains(
            self.editor.tracks,
            |track| self.track_volume_db(track),
            |track| self.track_is_audible(track),
        )
    }

    /// mixer で変わったぶんを live mix へ送る。
    ///
    /// mixer の値が変わりうる経路すべて（音量キー・solo・HTTP・project 読み込み）が通る
    /// `sync_playback_mml_state()` から呼ばれる。差分が無ければ何もしない。
    ///
    /// **演奏中しか送らない。** 「演奏していないときに送っても害は無い」わけではなく、
    /// `set_live_instance_gain_db` は `ensure_started_for_fast_midi()` を通るので、
    /// **止まっているサーバーがあれば起動してしまう**（実装を読んで確かめた）。
    /// 編集しているだけのユーザーの背後で play server が立ち上がるのは行き過ぎなので、
    /// 演奏開始時にまとめて送る形（[`super::live_cache::LiveCachePlayLoop`]）と
    /// 組み合わせてある。
    pub(crate) fn sync_live_track_gains(&self) {
        if self.cfg.realtime_audio_backend != RealtimeAudioBackend::CachePlayer {
            return;
        }
        if *self.playback.play_state.lock().unwrap() != DawPlayState::Playing {
            return;
        }
        let Some(play_server) = self.playback.realtime_play_server.as_ref() else {
            return;
        };
        let desired = self.desired_live_track_gains();
        let mut sent = self.playback.live_track_gains.lock().unwrap();
        let changed = changed_live_track_gains(&sent, &desired);
        if changed.is_empty() {
            return;
        }
        send_live_track_gains(play_server, &changed, &self.log_lines);
        *sent = desired;
    }
}

#[cfg(test)]
mod tests;
