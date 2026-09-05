//! `CachePlayer` backend の演奏ループ。
//!
//! セルの render キャッシュ（WAV）を、play server の組み込みプラグイン cache-player へ
//! 載せて live mix で鳴らす。1 演奏 track = 1 live instance で、小節が変わるたびに
//! その小節のキャッシュ WAV を鳴らす。
//!
//! rodio 経路（`InProcess`）と違い、**gain は mix の直前にサーバー側で掛かる**ので
//! mixer の音量変更が即座に効く。rodio 経路は「チャンクを append する瞬間」に
//! 振幅を焼き込むため、実測 2.4 秒（＝ 1 小節）遅れていた。
//!
//! ## 先読み（小節 N を鳴らしている最中に小節 N+1 を載せる）
//!
//! cache-player は同時に `SLOT_COUNT` 本の WAV を持てる。小節 index `N` の WAV は
//! スロット `N % SLOT_COUNT` へ載せ、その小節は note number `60 + (N % SLOT_COUNT)` で
//! 鳴らす（対応は [`cues::measure_slot`] と [`cues::note_on_events`]）。隣り合う小節が
//! 必ず別スロットになるので、**鳴っている音を壊さずに次の小節を載せておける。**
//!
//! こうする前は、小節境界に到達してから初めて state load を出していた。その応答待ち
//! （7 track で 100〜130ms）がまるごと小節の頭の無音になり、しかも 1 track ずつ順に
//! 返ってくるので track ごとにバラバラの時刻で前の小節の音が切れていた。
//!
//! **ループの長さが奇数のときは、末尾の小節と先頭の小節が同じスロットになる**
//! （3 小節なら meas3 も meas1 もスロット 0）。それでも壊れないのは、鳴っている voice が
//! 自分の握った音源を鳴らし続けるから（play server 側 Stage 2）。スロットを差し替えても
//! meas3 の余韻は切れず、境界の note on はちょうど載せ替えた meas1 を鳴らす。
//!
//! ## 発音位置は timeline で決める（実時刻ではない）
//!
//! note on は「届いた瞬間のオーディオブロックで鳴らせ」ではなく、**演奏開始からの
//! 絶対秒**で予約する（[`timeline::MeasureTimeline`]）。予約は先読みと同じ場所、
//! つまり 1 つ前の小節の中で出す。順序は必ず
//! **(a) スロットへロード → (b) その小節の note on を予約**。
//!
//! こうする前は小節境界で `send_live_events`（`offset_frames: 0`）を投げていたので、
//! 発音位置が小節ごとに −42.7〜+21.3ms ずれていた（振れ幅 64ms = 3 オーディオブロック）。
//!
//! **先読みが外れた小節（AB リピートや小節数を演奏中に変えたとき）は、予約済みの
//! note on を取り消せない。** 予約はサーバーのレンダークロックより先に届いていて、
//! 食い違いに気づくのは境界に着いてからだから。その 1 小節だけ「予測していた小節の
//! 頭が一瞬鳴り、その直後に本当の小節が始まる」形になる。ログの `preload=miss` が目印。
//!
//! ## 鳴らさないもの
//!
//! **キャッシュ WAV がまだ無い小節は無音のまま。** 直前のキャッシュを鳴らし続けたり、
//! その場で live render したりはしない（承認済みの設計判断）。「まだ出来ていない」ことが
//! 耳で分かるほうがよい、という判断で、`prepare_live_patch` も note on も送らない。

mod cues;
mod measure_log;
mod send;
mod timeline;

use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use cmrt_realtime_play::RealtimePlayServerSupervisor;

use super::{
    current_play_measure_index, effective_measure_count, following_measure_index,
    format_playback_measure_advance_log, format_playback_measure_resolution_log,
    live_gain::{send_live_track_gains, LiveTrackGain},
    measure_duration, wait_until_or_stop, DawApp, DawPlayState, DawPlaybackStartupState,
    PlayPosition,
};
use crate::{cache::cache_wav_path, AbRepeatState, WorkspaceKind};

pub(crate) use cues::{measure_live_cues, measure_slot};
pub(crate) use measure_log::format_live_cache_measure_log;
pub(crate) use send::MeasureSendTiming;
use send::{prepare_measure_cues, send_measure_note_on, PreloadedMeasure};
use timeline::MeasureTimeline;

/// その小節で実際に鳴らせるキャッシュ WAV を行ごとに引く。
fn ready_cache_wav_for_measure(
    workspace_kind: WorkspaceKind,
    measure_index: usize,
    row: usize,
) -> Option<PathBuf> {
    // キャッシュのファイル名は 1 始まりの小節番号で付く。演奏ループが持つ
    // `measure_index` は 0 始まりなので +1 する。
    let path = cache_wav_path(workspace_kind, row, measure_index + 1)?;
    path.is_file().then_some(path)
}

/// 1 つ前の小節の中で「スロットへ載せて・note on を予約して」おいた小節。
///
/// `at` を一緒に持ち回るのが要点。境界に着いてから位置を計算し直すと、
/// **予約した位置と表示・待ちの基準がずれる**。
struct ScheduledMeasure {
    measure: PreloadedMeasure,
    /// timeline 原点からのフレーム数。
    at: u64,
}

/// 小節境界で演奏スレッドが止まっていた時間の内訳。
///
/// 予約が当たった小節では**両方 0**。つまり境界では 1 バイトも送っていない。
/// ここが 0 でない小節は、その合計がそのまま小節の頭の遅れになる。
#[derive(Clone, Copy, Debug, Default)]
struct BoundaryWork {
    prepare: Duration,
    note_on: Duration,
}

impl BoundaryWork {
    /// 境界で何もしなかった＝先読みと予約が当たっていた。
    fn is_preloaded(self) -> bool {
        self.prepare.is_zero() && self.note_on.is_zero()
    }
}

/// 演奏ループが要るものだけを集めた実行単位。
///
/// **`DawApp` を持たない**のが要点。`DawApp` は起動にスレッド 2 本とグローバルな
/// http state とユーザーのファイルを要求するのでテストから組み立てられないが、
/// ここにある Arc は全部テストからそのまま作れる。キャッシュ WAV の引き方も
/// [`Self::ready_cache_wav`] で差し替えられるので、**実サーバーへ繋いだうえで
/// ループを丸ごと走らせるテスト**が書ける（ユーザーの実キャッシュを汚さずに）。
pub(crate) struct LiveCachePlayLoop {
    pub(crate) play_server: Arc<RealtimePlayServerSupervisor>,
    pub(crate) play_state: Arc<Mutex<DawPlayState>>,
    pub(crate) play_position: Arc<Mutex<Option<PlayPosition>>>,
    pub(crate) ab_repeat: Arc<Mutex<AbRepeatState>>,
    pub(crate) measure_mmls: Arc<Mutex<Vec<String>>>,
    pub(crate) measure_samples: Arc<Mutex<usize>>,
    pub(crate) log_lines: Arc<Mutex<VecDeque<String>>>,
    pub(crate) sample_rate: u32,
    /// timeline へ載せるテンポ。cache-player は使わないが、**サーバーが CLAP の
    /// transport として全 instance へ配る**ので、tempo-sync するプラグインを
    /// 混ぜたときに効いてくる。
    pub(crate) tempo_bpm: f64,
    /// 1 小節の拍数（分母は 4 固定）。用途は `tempo_bpm` と同じ。
    pub(crate) beat_numerator: u16,
    pub(crate) tracks: usize,
    /// `(0 始まりの小節 index, グリッドの行 index)` → いま鳴らせるキャッシュ WAV。
    #[allow(clippy::type_complexity)]
    pub(crate) ready_cache_wav: Arc<dyn Fn(usize, usize) -> Option<PathBuf> + Send + Sync>,
    /// 演奏開始時に送る mixer の gain（演奏開始を決めたときのスナップショット）。
    pub(crate) initial_track_gains: Vec<LiveTrackGain>,
    /// live mix へ最後に送った gain。演奏中の mixer 操作と共有する。
    pub(crate) sent_track_gains: Arc<Mutex<Vec<LiveTrackGain>>>,
    /// 最初の音が出るまでの進み具合の置き場。ここへ書いた値を描画スレッドが
    /// 中央 overlay として読む。演奏中は触らない（**1 小節目だけ**の話）。
    pub(crate) startup: DawPlaybackStartupState,
}

impl LiveCachePlayLoop {
    /// 停止されるまで小節を進めながら鳴らし続ける。呼んだスレッドを占有する。
    pub(crate) fn run(self, start_measure_index: usize) {
        // サーバーの auto gain は「実際に鳴った音の RMS」から補正値を決めるので、
        // mixer で下げたぶんを打ち消しに来る。DAW の mixer は「ユーザーが決めた音量」
        // なので、live 演奏のあいだは切っておく。
        // grid sequencer は自分の演奏開始時に必ず on へ戻す（`prepare_connection`）ので、
        // ここで off のまま終わっても grid sequencer 側の auto gain は壊れない。
        // **ここが play server の起動待ちの実体。** `set_live_auto_gain_enabled` は
        // `with_fast_client` 経由で `ensure_started_for_fast_midi` を通るので、
        // サーバーがまだ立っていなければこの 1 行で数秒ブロックする
        // （実測 1747ms / cold 6559ms）。抜けた時点でサーバーは立っている。
        if let Err(error) = self.play_server.set_live_auto_gain_enabled(false) {
            crate::append_log_line(
                &self.log_lines,
                format!("live-cache: auto gain off failed error=\"{error:#}\""),
            );
        }
        self.apply_initial_track_gains();

        // 演奏 1 回につき 1 本。**演奏中に張り直してはいけない**（サーバー側は
        // プラグインの状態もサンプルクロックの原点も戻すフルリセットになる）。
        //
        // ここではまだサーバーのクロックは動かない。動き出すのは 1 小節目のロードの
        // あとに呼ぶ `start_clock`（下の `_ =>` の腕）。**逆に `begin` を後ろへ
        // ずらしてはいけない**（`BeginLiveTimeline` は `banks.reset_all()` を伴うので、
        // 先に載せた state load が消える）。
        let mut timeline = MeasureTimeline::begin(
            &self.play_server,
            self.sample_rate,
            self.tempo_bpm,
            self.beat_numerator,
            &self.log_lines,
        );

        let mut measure_index = start_measure_index;
        // 1 つ前の小節の中で「載せて・予約して」おいた小節。演奏開始の 1 小節目だけ空。
        let mut scheduled: Option<ScheduledMeasure> = None;
        // 「音が鳴るまで」overlay を出しているあいだだけ true。1 小節目を鳴らす
        // 手配が済んだら下ろす。**先読みが外れた小節では出さない**（もう鳴っている）。
        let mut waiting_for_first_sound = true;

        'outer: loop {
            if *self.play_state.lock().unwrap() != DawPlayState::Playing {
                break;
            }

            let mmls = self.measure_mmls.lock().unwrap().clone();
            let measure_samples = *self.measure_samples.lock().unwrap();
            let effective_count = match effective_measure_count(&mmls) {
                Some(n) => n,
                None => break 'outer,
            };
            let ab_repeat_range =
                (*self.ab_repeat.lock().unwrap()).normalized_range(effective_count);
            let current_measure_index =
                current_play_measure_index(measure_index, effective_count, ab_repeat_range);
            let measure_duration = measure_duration(measure_samples, self.sample_rate);
            // `measure_samples` はステレオのインターリーブ済み要素数なので、
            // フレーム数はその半分。timeline はフレームで積む（`timeline.rs` の doc）。
            let measure_frames = (measure_samples / 2) as u64;
            crate::append_log_line(
                &self.log_lines,
                format_playback_measure_resolution_log(
                    measure_index,
                    current_measure_index,
                    effective_count,
                ),
            );

            // (a) この小節。予約が当たっていれば、境界では 1 バイトも送らない。
            let (measure, at, boundary) = match scheduled.take() {
                Some(ready) if ready.measure.measure_index == current_measure_index => {
                    (ready.measure, ready.at, BoundaryWork::default())
                }
                // 演奏開始の 1 小節目か、演奏中に AB リピート・小節数が変わって
                // 先読みした小節と食い違ったとき。ここだけは境界で載せて張り直す。
                _ => {
                    let measure =
                        self.load_measure_reporting(current_measure_index, waiting_for_first_sound);
                    let prepare = measure.elapsed;
                    // **ロードが終わってからサーバーのクロックを起こす。** 起こすまで
                    // サーバーは眠っていてクロックが 1 サンプルも進まないので、
                    // ここが timeline の原点とサーバーの原点が揃う唯一の点。
                    // 先に起こすと、1 小節目のロード（実測 3.10 秒）のあいだクロックは
                    // 0.26 秒しか進まず、差の 2.7 秒ぶん演奏ループが先行する。
                    // 先行したぶん先読みが**まだ鳴っていない小節のスロットを踏み潰し**、
                    // 違う小節が鳴る
                    // （`docs/adr/0012-live-clock-drift-is-absorbed-not-eliminated.md` の症状 B）。
                    // 2 度目以降（先読みが外れた小節）は何もしない。
                    timeline.start_clock(&self.play_server, &self.log_lines);
                    let at = timeline.restart_at(Instant::now(), measure_frames);
                    let note_on = send_measure_note_on(
                        &self.play_server,
                        &timeline,
                        &measure,
                        at,
                        &self.log_lines,
                    );
                    (measure, at, BoundaryWork { prepare, note_on })
                }
            };

            // 1 小節目の手配が済んだ＝これ以上は待たずに音が出る。overlay を消す。
            if waiting_for_first_sound {
                self.startup.finish();
                waiting_for_first_sound = false;
            }

            // 表示も待ちも timeline の位置から引く。`Instant::now()` を基準にすると
            // 待ちのオーバーシュートが毎小節積もり、音（＝timeline）とずれていく。
            let measure_start = timeline.instant_of(at);
            *self.play_position.lock().unwrap() = Some(PlayPosition {
                measure_index: current_measure_index,
                measure_start,
                measure_duration,
            });

            // (b) 鳴らしている最中に、次の小節を別スロットへ載せて note on を予約する。
            //     **`+1` を自前で計算しないこと。** AB リピート・小節数変更・ループ端は
            //     `following_measure_index` だけが正しく畳める。
            let next_measure_index =
                following_measure_index(current_measure_index, effective_count, ab_repeat_range);
            // 位置はロードの**前**に取る。ロードは 100〜600ms 塞ぐので、後で取ると
            // 「いまから」の下限に引っ掛かってグリッドを張り直してしまう。
            let next_at = timeline.reserve(Instant::now(), measure_frames);
            let ahead = self.load_measure(next_measure_index);
            let next_note_on = send_measure_note_on(
                &self.play_server,
                &timeline,
                &ahead,
                next_at,
                &self.log_lines,
            );
            let timing = MeasureSendTiming {
                preloaded: boundary.is_preloaded(),
                at_frames: at,
                prepare: boundary.prepare,
                note_on: boundary.note_on,
                preload_next: ahead.elapsed,
                note_on_next: next_note_on,
            };
            crate::append_log_line(
                &self.log_lines,
                format_live_cache_measure_log(
                    current_measure_index,
                    next_measure_index,
                    &measure.cues,
                    timing,
                ),
            );
            scheduled = Some(ScheduledMeasure {
                measure: ahead,
                at: next_at,
            });

            let next_measure_start = measure_start + measure_duration;
            if !wait_until_or_stop(&self.play_state, next_measure_start) {
                break 'outer;
            }

            let next_mmls = self.measure_mmls.lock().unwrap().clone();
            let next_effective_count = match effective_measure_count(&next_mmls) {
                Some(n) => n,
                None => break 'outer,
            };
            let next_ab_repeat_range =
                (*self.ab_repeat.lock().unwrap()).normalized_range(next_effective_count);
            let advanced_measure_index = following_measure_index(
                current_measure_index,
                next_effective_count,
                next_ab_repeat_range,
            );
            crate::append_log_line(
                &self.log_lines,
                format_playback_measure_advance_log(
                    current_measure_index,
                    advanced_measure_index,
                    next_effective_count,
                ),
            );
            measure_index = advanced_measure_index;
        }

        // 1 音も鳴らずに抜けた場合（演奏する中身が無い・すぐ止められた）も
        // overlay を残さない。
        self.startup.finish();
        let _ = self.play_server.stop_live_all();
        // 次の演奏では全 track ぶん送り直す。サーバーが再起動していても食い違わない。
        self.sent_track_gains.lock().unwrap().clear();
        let mut state = self.play_state.lock().unwrap();
        if *state == DawPlayState::Playing {
            *state = DawPlayState::Idle;
            drop(state);
            *self.play_position.lock().unwrap() = None;
            crate::append_log_line(&self.log_lines, "play: finished");
        }
    }

    /// 1 小節ぶんのキャッシュ WAV を、その小節のスロットへ載せる。
    ///
    /// 載せ先は `measure_index % SLOT_COUNT` に決まっているので、**隣り合う小節は
    /// 必ず別スロット**になる。鳴っている voice は自分が握った音源を鳴らし続けるので
    /// （play server 側 Stage 2）、ここで差し替えても前の小節の余韻は切れない。
    fn load_measure(&self, measure_index: usize) -> PreloadedMeasure {
        self.load_measure_reporting(measure_index, false)
    }

    /// `report_startup` が true のときだけ、載せ終えた本数を
    /// [`DawPlaybackStartupState`] へ報告する（＝「音が鳴るまで」overlay へ出す）。
    ///
    /// **報告するのは演奏開始の 1 小節目だけ。** 2 小節目以降は鳴っている最中の
    /// 先読みなので、進捗を出すと「鳴っているのに読み込み中」に見える。
    fn load_measure_reporting(
        &self,
        measure_index: usize,
        report_startup: bool,
    ) -> PreloadedMeasure {
        let cues = measure_live_cues(self.tracks, |row| {
            (self.ready_cache_wav)(measure_index, row)
        });
        if report_startup {
            self.startup.begin_first_measure(cues.cues.len());
        }
        prepare_measure_cues(
            &self.play_server,
            measure_index,
            measure_slot(measure_index),
            cues,
            &self.log_lines,
            &mut |loaded| {
                if report_startup {
                    self.startup.note_measure_loaded(loaded);
                }
            },
        )
    }

    /// 演奏開始時に mixer の gain をまとめて送る。
    ///
    /// 記録が空でないときは**何もしない**。演奏開始と同時に mixer が触られていて、
    /// UI 側（`sync_live_track_gains`）がもっと新しい値を送り終えているという意味なので、
    /// ここで開始時のスナップショットを送ると古い値で上書きしてしまう。
    /// 記録の Mutex を握ったまま送るので、UI 側とどちらが先でも取り違えは起きない。
    fn apply_initial_track_gains(&self) {
        let mut sent = self.sent_track_gains.lock().unwrap();
        if !sent.is_empty() {
            return;
        }
        send_live_track_gains(
            &self.play_server,
            &self.initial_track_gains,
            &self.log_lines,
        );
        sent.clone_from(&self.initial_track_gains);
    }
}

impl DawApp {
    pub(super) fn start_play_from_measure_via_cache_player(&self, start_measure_index: usize) {
        let Some(play_server) = self.playback.realtime_play_server.as_ref().cloned() else {
            self.append_log_line("play: realtime play server is not initialized");
            *self.playback.play_state.lock().unwrap() = DawPlayState::Idle;
            self.playback.startup.finish();
            return;
        };

        let workspace_kind = self.workspace_kind;
        let play_loop = LiveCachePlayLoop {
            play_server,
            play_state: Arc::clone(&self.playback.play_state),
            play_position: Arc::clone(&self.playback.position),
            ab_repeat: Arc::clone(&self.playback.ab_repeat),
            measure_mmls: Arc::clone(&self.playback.measure_mmls),
            measure_samples: Arc::clone(&self.playback.measure_samples),
            log_lines: Arc::clone(&self.log_lines),
            sample_rate: self.cfg.sample_rate as u32,
            tempo_bpm: self.tempo_bpm(),
            beat_numerator: self.beat_numerator().clamp(1, u32::from(u16::MAX)) as u16,
            tracks: self.editor.tracks,
            ready_cache_wav: Arc::new(move |measure_index, row| {
                ready_cache_wav_for_measure(workspace_kind, measure_index, row)
            }),
            initial_track_gains: self.desired_live_track_gains(),
            sent_track_gains: Arc::clone(&self.playback.live_track_gains),
            startup: self.playback.startup.clone(),
        };

        std::thread::spawn(move || play_loop.run(start_measure_index));
    }
}

#[cfg(test)]
mod tests;
