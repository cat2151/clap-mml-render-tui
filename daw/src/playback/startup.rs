//! 「演奏を始めてから最初の音が出るまで」の進み具合。
//!
//! ## なぜ持つか（実測）
//!
//! release ビルド・ユーザーの実キャッシュで DAW を cold start したときの
//! `log.txt`（2026-09-03 19:30:38〜19:30:42）:
//!
//! ```text
//! 19:30:39 play: start
//! 19:30:40 play-server: prewarm ms=1747 result=ok      ← ここまで play server の起動待ち
//! 19:30:40 live-cache: timeline begin id=1 result=ok
//! 19:30:41 shm-patch-prepare success instance=0 elapsed_ms=1229
//! 19:30:42 shm-patch-prepare success instance=6 elapsed_ms=11
//! 19:30:42 live-cache: timeline clock start id=1 result=ok   ← ここで初めて音が出る
//! 19:30:42 meas1: ... prepare_ms=2101.9
//! ```
//!
//! つまり待ちは 2 段階で、**どちらも数百 ms〜数秒**（play server は cold で 6559ms、
//! 1 小節目のロードは OS のファイルキャッシュ次第で 72ms〜3614ms）。
//! この間、画面には何も出ていなかった。
//!
//! ## 誰が書いて誰が読むか
//!
//! 書くのは演奏スレッド（[`super::live_cache::LiveCachePlayLoop::run`]）。
//! 読むのは描画スレッド。だから `Arc<Mutex<_>>` で共有する。
//!
//! **play server の「何本目の instance か」はここには持たない。** その数は
//! supervisor が子プロセスの stderr から拾って持っているので
//! （`cmrt_realtime_play::RealtimePlayServerSupervisor::startup_progress`）、
//! 二重に持つと必ず食い違う。ここが持つのは「いまどの段階か」だけ。

use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

/// 最初の音が出るまでの、いまの段階。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DawPlaybackStartupStage {
    /// play server（CLAP instance 14 本）の起動待ち。
    ///
    /// `first_measure_follows` は、このあと 1 小節目のキャッシュロードが続くか。
    /// `CachePlayer` backend では true、小節ごとに SMF を投げる `PlayServer`
    /// backend では false（あちらはキャッシュを載せない）。
    PlayServer { first_measure_follows: bool },
    /// 1 小節目のキャッシュ WAV を live instance へ載せている最中。
    FirstMeasure { loaded: usize, total: usize },
}

/// 段階と、演奏開始を決めた時刻。
#[derive(Clone, Copy, Debug)]
pub(crate) struct DawPlaybackStartup {
    pub(crate) stage: DawPlaybackStartupStage,
    pub(crate) started: Instant,
}

/// 演奏スレッドが書き、描画スレッドが読む共有スロット。
///
/// `None` は「待っていない」＝ overlay を出さない状態。
#[derive(Clone, Default)]
pub(crate) struct DawPlaybackStartupState(Arc<Mutex<Option<DawPlaybackStartup>>>);

impl DawPlaybackStartupState {
    /// play server の起動待ちに入る。
    ///
    /// **演奏スレッドを spawn する前に、呼び出し元のスレッドで呼ぶこと。**
    /// 演奏スレッドは最初の IPC でそのまま数秒ブロックするので、あちらで
    /// 印を付けてからでは最初の 1 フレームに間に合わない。
    pub(crate) fn begin(&self, first_measure_follows: bool) {
        *self.0.lock().unwrap() = Some(DawPlaybackStartup {
            stage: DawPlaybackStartupStage::PlayServer {
                first_measure_follows,
            },
            started: Instant::now(),
        });
    }

    /// 1 小節目のロードに入る。`started` は引き継ぐ（経過時間は演奏開始からの通し）。
    pub(crate) fn begin_first_measure(&self, total: usize) {
        let mut slot = self.0.lock().unwrap();
        let started = slot.map_or_else(Instant::now, |startup| startup.started);
        *slot = Some(DawPlaybackStartup {
            stage: DawPlaybackStartupStage::FirstMeasure { loaded: 0, total },
            started,
        });
    }

    /// 1 小節目のうち `loaded` 本まで載せ終えた。
    ///
    /// 段階が変わっていたら（＝停止して次の演奏が始まっていたら）何もしない。
    /// 遅れて届いた報告で、次の演奏の進捗を巻き戻さないため。
    pub(crate) fn note_measure_loaded(&self, loaded: usize) {
        let mut slot = self.0.lock().unwrap();
        if let Some(DawPlaybackStartup {
            stage:
                DawPlaybackStartupStage::FirstMeasure {
                    loaded: current, ..
                },
            ..
        }) = slot.as_mut()
        {
            *current = loaded;
        }
    }

    /// 待ちが終わった（音が出た、または演奏が始まらずに終わった）。
    pub(crate) fn finish(&self) {
        *self.0.lock().unwrap() = None;
    }

    /// 描画スレッドが読む一枚。
    pub(crate) fn snapshot(&self) -> Option<DawPlaybackStartup> {
        *self.0.lock().unwrap()
    }
}

#[cfg(test)]
mod tests;
