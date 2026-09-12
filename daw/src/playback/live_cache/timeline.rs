//! 小節の発音位置を、サーバーのサンプルクロックの上に置く時間割。
//!
//! 小節 N の発音位置を演奏開始からの絶対位置で決め、サーバーがその位置をサンプルへ丸めて
//! 予約する（play server 側 `BlockScheduler::schedule`）。送信が数十 ms ぶれても発音位置は
//! 1 サンプルも動かない。「届いた瞬間のブロックで鳴らせ」と送っていたころのジッタは `docs/adr/0016`。
//!
//! 位置はサンプル（フレーム）で積む。秒や `Duration` で積むと、サーバーが
//! `round(秒 × サンプルレート)` で戻すときに小節ごと ±1 サンプル揺れる（小節長がナノ秒で
//! 割り切れないと必ず起きる）。フレームで積んで送る直前に `frames / sample_rate` で秒へ直せば、
//! サーバーの `round()` は必ず元のフレーム数へ戻る（`f64` は 2^53 まで整数を正確に持てる）。
//!
//! 予約はサーバーのレンダークロック（出力リングのぶん実時間より先を走る）より先でなければ
//! ならない。「いま」を指定すると既に過ぎていてブロック頭へクランプされ、ジッタが戻る。
//! 通常の経路は 1 小節ぶん先に予約するので余裕があり、[`MAX_LEAD`] が効くのは演奏開始と、
//! 先読みが外れて境界で組み直すときだけ。
//!
//! 原点は「サーバーのクロックが動き出した瞬間」に置く。ここの外挿は「サーバーのサンプル
//! クロックは実時間どおり進む」前提で、state load 中はサーバーが render を回さないので
//! 前提が崩れ、演奏ループが先行してまだ鳴っていない小節のスロットを先読みが踏み潰す
//! （`docs/adr/0012`）。だから [`Self::begin`] と [`Self::start_clock`] を分け、
//! **begin → 1 小節目の state load → `start_clock` → [`Self::restart_at`]** の順で呼ぶ。
//! `start_clock` を呼ぶまでサーバーは眠っていて（play server 側 `worker.rs` の
//! `waiting_for_timeline_events`）クロックは 1 サンプルも進まないので、起こした瞬間を原点に
//! すれば両者が揃う。`begin` のほうを後ろへずらしてはいけない。`BeginLiveTimeline` は
//! `banks.reset_all()` を伴うので、先に載せておいた state load が消える。

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use cmrt_realtime_play::{LiveTimelineConfig, RealtimePlayServerSupervisor, TimelineMidiEvent};

/// timeline id。**0 は「timeline 無し」の意味**なのでサーバーに弾かれる。
static NEXT_TIMELINE_ID: AtomicU64 = AtomicU64::new(1);

/// 「いまから鳴らす」ときに空ける先行時間の上限。出力リングの先読みと IPC の往復を吸収できればよい。
/// 削ると演奏開始の 1 音目が late になり、`timing_metrics().late_events_total` が 0 でなくなる。
const MAX_LEAD: Duration = Duration::from_millis(250);

/// 演奏中ずっと 1 本だけ張られる timeline と、次に予約する小節の位置。
///
/// [`Self::begin`] を演奏中に呼び直してはいけない。サーバー側の `BeginLiveTimeline` は
/// フルリセットで、呼んだ瞬間に音が切れる。テンポを変えたいだけなら `set_live_tempo`。
pub(crate) struct MeasureTimeline {
    id: u64,
    sample_rate: u32,
    /// timeline のフレーム 0 に相当する実時刻。[`Self::start_clock`] が置く。
    /// サーバーが起こしのイベントを処理するのはこれより少し後なので、
    /// **実際の原点はここより後ろ**（＝予約はさらに余裕がある）。
    origin: Instant,
    /// 次に予約する小節の発音位置（原点からのフレーム数）。
    next_at: u64,
    /// [`Self::start_clock`] が済んでいるか。2 度起こさないためだけに持つ。
    clock_started: bool,
}

impl MeasureTimeline {
    /// timeline を 1 本張る。演奏 1 回につき 1 度だけ。
    /// ここではまだサーバーのクロックは動かない。動き出すのは [`Self::start_clock`]。
    pub(crate) fn begin(
        play_server: &RealtimePlayServerSupervisor,
        sample_rate: u32,
        tempo_bpm: f64,
        beat_numerator: u16,
        log_lines: &Arc<Mutex<VecDeque<String>>>,
    ) -> Self {
        let id = NEXT_TIMELINE_ID.fetch_add(1, Ordering::Relaxed).max(1);
        let result = play_server.begin_live_timeline(LiveTimelineConfig {
            timeline_id: id,
            sample_rate_hz: f64::from(sample_rate),
            tempo_bpm,
            time_signature_numerator: beat_numerator.max(1),
            time_signature_denominator: 4,
        });
        crate::append_log_line(
            log_lines,
            match &result {
                Ok(()) => format!(
                    "live-cache: timeline begin id={id} sample_rate={sample_rate} \
                     bpm={tempo_bpm} beats={beat_numerator} result=ok"
                ),
                // ここが error だと **note on が 1 つも鳴らない**（サーバーは timeline が
                // 無いと timeline MIDI を捨てる）。無音の原因がここだと分かるように残す。
                Err(error) => {
                    format!("live-cache: timeline begin failed id={id} error=\"{error:#}\"")
                }
            },
        );
        Self {
            id,
            sample_rate,
            // `start_clock` で置き直す。ここは「まだ起こしていない」あいだの仮。
            origin: Instant::now(),
            next_at: 0,
            clock_started: false,
        }
    }

    /// サーバーのサンプルクロックを起こし、その瞬間を原点にする。
    ///
    /// サーバーは最初の timeline イベントが届くまで render せずに眠るので、音の出ないイベント
    /// （note off）を 1 つ置いて起こす。cache-player は note on しか見ないので音は出ない
    /// （play server 側 `cache-player/src/lib.rs` の `0x90 && velocity != 0`）。
    ///
    /// 呼ぶのは 1 小節目の state load が終わってから。ロード中に起こすと、そのぶん
    /// 「クロックは進んだことになっているのに実際は進んでいない」状態で演奏ループが走り出す。
    /// 2 度目以降の呼び出しは何もしない。
    pub(crate) fn start_clock(
        &mut self,
        play_server: &RealtimePlayServerSupervisor,
        log_lines: &Arc<Mutex<VecDeque<String>>>,
    ) {
        if self.clock_started {
            return;
        }
        self.clock_started = true;
        let id = self.id;
        let result = play_server.send_timeline_events(&[TimelineMidiEvent {
            timeline_id: id,
            instance_id: 0,
            timeline_seconds: 0.0,
            message: [0x80, 0, 0],
        }]);
        // **原点はここ。** サーバーはこのイベントを受け取ってから render を始めるので、
        // 実際の原点はこれより少しあと（＝予約はさらに余裕がある側へずれる）。
        self.origin = Instant::now();
        crate::append_log_line(
            log_lines,
            match &result {
                Ok(_) => format!("live-cache: timeline clock start id={id} result=ok"),
                // 起こせないと **1 音も鳴らない**（サーバーは眠ったまま）。
                Err(error) => {
                    format!("live-cache: timeline clock start failed id={id} error=\"{error:#}\"")
                }
            },
        );
    }

    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    /// 予約位置（フレーム）を、サーバーへ渡す絶対秒へ直す。
    ///
    /// **丸めはサーバー側**（`SampleRate::seconds_to_sample` が `round()`）。ここで
    /// 割った値をサーバーが掛け戻すと、必ず元のフレーム数へ戻る。
    pub(crate) fn seconds_of(&self, at: u64) -> f64 {
        at as f64 / f64::from(self.sample_rate)
    }

    /// 予約位置を実時間の長さへ直す（ログ表示と、演奏位置の計算に使う）。
    pub(crate) fn duration_of(&self, at: u64) -> Duration {
        Duration::from_secs_f64(self.seconds_of(at))
    }

    /// 予約位置に対応する実時刻。演奏位置の表示と小節境界の待ちに使う。
    pub(crate) fn instant_of(&self, at: u64) -> Instant {
        self.origin + self.duration_of(at)
    }

    /// 次の小節の発音位置を取り、その先へ 1 小節ぶん進める。
    ///
    /// **通常の経路はここしか通らない。** 返る位置は前の小節のちょうど 1 小節後なので、
    /// 発音位置は小節長ちょうどの間隔で並ぶ。
    ///
    /// 予約位置が「いま」に近すぎるときだけ [`Self::restart_at`] と同じ張り直しをする。
    /// 演奏スレッドが 1 小節ぶん近く遅れたときの保険で、正常時には起きない。
    pub(crate) fn reserve(&mut self, now: Instant, measure_frames: u64) -> u64 {
        if self.next_at < self.earliest(now, measure_frames) {
            return self.restart_at(now, measure_frames);
        }
        let at = self.next_at;
        self.next_at = at.saturating_add(measure_frames);
        at
    }

    /// グリッドを「いまから」張り直す。
    ///
    /// 使うのは演奏開始の 1 小節目と、先読みが外れて小節境界で組み直すときだけ。
    /// [`Self::reserve`] をそのまま使うと、外れた小節ぶん先の位置が返って
    /// 1 小節まるごと無音になる。
    pub(crate) fn restart_at(&mut self, now: Instant, measure_frames: u64) -> u64 {
        let at = self.earliest(now, measure_frames);
        self.next_at = at.saturating_add(measure_frames);
        at
    }

    /// いま予約してよい最も早い位置（フレーム）。
    ///
    /// 先行時間は [`MAX_LEAD`] だが、小節が短いときは小節長の半分まで縮める。
    /// 縮めないと、小節長 250ms のような設定で「次の小節の予約位置」より
    /// 「いまから MAX_LEAD 後」のほうが後ろになり、毎小節グリッドを張り直してしまう。
    fn earliest(&self, now: Instant, measure_frames: u64) -> u64 {
        let elapsed = self.frames_of(now.saturating_duration_since(self.origin));
        let lead = self.frames_of(MAX_LEAD).min(measure_frames / 2);
        elapsed.saturating_add(lead)
    }

    fn frames_of(&self, elapsed: Duration) -> u64 {
        (elapsed.as_secs_f64() * f64::from(self.sample_rate)) as u64
    }

    /// 実サーバーを起こさずにグリッドの計算だけを見るための組み立て。
    #[cfg(test)]
    pub(crate) fn for_test(origin: Instant, sample_rate: u32) -> Self {
        Self {
            id: 1,
            sample_rate,
            origin,
            next_at: 0,
            // 実サーバーが無いので起こしようがない。起こし済みとして扱う。
            clock_started: true,
        }
    }
}
