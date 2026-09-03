//! `CachePlayer` backend の演奏ループ。
//!
//! セルの render キャッシュ（WAV）を、play server の組み込みプラグイン cache-player へ
//! 載せて live mix で鳴らす。1 演奏 track = 1 live instance で、小節が変わるたびに
//! その小節のキャッシュ WAV を差し替えて note on する。
//!
//! rodio 経路（`InProcess`）と違い、**gain は mix の直前にサーバー側で掛かる**ので
//! mixer の音量変更が即座に効く。rodio 経路は「チャンクを append する瞬間」に
//! 振幅を焼き込むため、実測 2.4 秒（＝ 1 小節）遅れていた。
//!
//! ## 鳴らさないもの
//!
//! **キャッシュ WAV がまだ無い小節は無音のまま。** 直前のキャッシュを鳴らし続けたり、
//! その場で live render したりはしない（承認済みの設計判断）。「まだ出来ていない」ことが
//! 耳で分かるほうがよい、という判断で、`prepare_live_patch` も note on も送らない。

use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use cmrt_realtime_play::{FastMidiEvent, InstanceId, RealtimePlayServerSupervisor};

use super::{
    current_play_measure_index, effective_measure_count, following_measure_index,
    format_playback_measure_advance_log, format_playback_measure_resolution_log,
    live_gain::{send_live_track_gains, LiveTrackGain},
    measure_duration, wait_until_or_stop, DawApp, DawPlayState, PlayPosition, FIRST_PLAYABLE_TRACK,
};
use crate::{
    cache::cache_wav_path, live_instance::live_instance_for_grid_row, AbRepeatState, WorkspaceKind,
};

/// cache-player へ「載っている WAV を先頭から鳴らせ」と伝える note on。
///
/// cache-player は音高を見ない（1 instance = 1 キャッシュ）ので値そのものに意味は無い。
/// ログを読むときに紛れないよう C4 / velocity 100 に固定してある。
const CACHE_PLAYER_NOTE_ON: [u8; 3] = [0x90, 60, 100];

/// ある小節で、ある行のキャッシュを鳴らすために送る 1 組。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LiveCacheCue {
    pub(crate) row: usize,
    pub(crate) instance: InstanceId,
    pub(crate) wav: PathBuf,
}

/// 1 小節ぶんの送信内容と、送らなかった行の内訳。
///
/// 送らなかった行を捨てずに持つのは、**無音が意図どおりか**をログで確かめるため。
/// 「キャッシュがまだ無い（`silent_rows`）」と「instance が足りない
/// （`rows_over_instance_limit`）」は原因が別物なので分けてある。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct MeasureLiveCues {
    pub(crate) cues: Vec<LiveCacheCue>,
    pub(crate) silent_rows: Vec<usize>,
    pub(crate) rows_over_instance_limit: Vec<usize>,
}

/// 1 小節ぶんの送信に掛かった実時間の内訳。
///
/// 2 つに分けてあるのは、**片方しか縮められない**ため。`prepare`（cache WAV の state load）
/// は 1 件ずつ応答待ちで送るしかないので track 数に比例して伸びるが、`note_on` は
/// 全 track を 1 コマンドへまとめられるので track 数に関係なく一定になる。
/// 小節の頭で音がずれて聞こえるかを決めるのは後者だけなので、混ぜて 1 つの数字にすると
/// 「ずれているのか、単に state load が重いだけなのか」が読めなくなる。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct MeasureSendTiming {
    /// 全 track ぶんの state load（cache WAV の差し替え）が終わるまでの実時間。
    ///
    /// `prepare_live_patch` は応答待ちでブロックするので、この値がそのまま
    /// 「小節の頭で演奏スレッドが止まっていた時間」になる。小節長（約 2.4 秒）と
    /// 並べれば占有率が読める（判断 3 の実測用）。
    pub(crate) prepare: Duration,
    /// 全 track ぶんの note on を送り終えるまでの実時間。
    ///
    /// 1 コマンドへまとめて投げるので、**最初の track と最後の track の note on の
    /// 間隔の上限**がこの値になる（サーバー側は 1 つのコマンドを 1 オーディオブロックで
    /// 処理するので、実際のずれは 0）。
    pub(crate) note_on: Duration,
}

/// 1 小節ぶんに live 経路へ送るものを組み立てる。
///
/// `ready_cache_wav` は「その行のキャッシュ WAV が**存在するなら**その絶対パス」を返す。
/// 実ファイルを見るのは呼び出し側の責務にしてあるので、この関数は実サーバーも
/// ファイルシステムも無しで単体テストできる。
pub(crate) fn measure_live_cues(
    tracks: usize,
    ready_cache_wav: impl Fn(usize) -> Option<PathBuf>,
) -> MeasureLiveCues {
    let mut cues = MeasureLiveCues::default();
    for row in FIRST_PLAYABLE_TRACK..tracks {
        let Some(instance) = live_instance_for_grid_row(row) else {
            cues.rows_over_instance_limit.push(row);
            continue;
        };
        match ready_cache_wav(row) {
            Some(wav) => cues.cues.push(LiveCacheCue { row, instance, wav }),
            None => cues.silent_rows.push(row),
        }
    }
    cues
}

/// 小節ごとの送信内容を 1 行にまとめる。
///
/// 例:
/// `meas3: live-cache sent=row2/i0,row4/i2 silent=row3 over_limit=- prepare_ms=1.5 note_on_ms=0.1`
///
/// `prepare_ms` と `note_on_ms` を分けて出すのは、**小節の頭の詰まり（state load）と
/// track 間のずれ（note on）を別々に読むため**。前者は track 数に比例して伸びるのが正常で、
/// 後者が伸びていたら「まとめて 1 コマンドで送る」形が壊れたということになる。
pub(crate) fn format_live_cache_measure_log(
    measure_index: usize,
    cues: &MeasureLiveCues,
    timing: MeasureSendTiming,
) -> String {
    let sent = join_or_dash(
        cues.cues
            .iter()
            .map(|cue| format!("row{}/i{}", cue.row, cue.instance)),
    );
    let silent = join_or_dash(cues.silent_rows.iter().map(|row| format!("row{row}")));
    let over_limit = join_or_dash(
        cues.rows_over_instance_limit
            .iter()
            .map(|row| format!("row{row}")),
    );
    format!(
        "meas{}: live-cache sent={sent} silent={silent} over_limit={over_limit} \
         prepare_ms={:.1} note_on_ms={:.1}",
        measure_index + 1,
        timing.prepare.as_secs_f64() * 1_000.0,
        timing.note_on.as_secs_f64() * 1_000.0,
    )
}

fn join_or_dash(items: impl Iterator<Item = String>) -> String {
    let joined = items.collect::<Vec<_>>().join(",");
    if joined.is_empty() {
        "-".to_string()
    } else {
        joined
    }
}

/// 小節の頭で全 track を一斉に鳴らすための note on を 1 バッチに組み立てる。
///
/// `offset_frames` は全部 0。サーバーは 1 コマンドぶんのイベントを**同じオーディオ
/// ブロックで**適用するので、これで track 間のずれが消える。
///
/// バッチの上限は `MAX_MIDI_MESSAGES`（128）で、cue は最大 16
/// （[`crate::live_instance::MAX_LIVE_TRACKS`]）なので分割は要らない。
pub(crate) fn note_on_events(cues: &[LiveCacheCue]) -> Vec<FastMidiEvent> {
    cues.iter()
        .map(|cue| FastMidiEvent {
            instance_id: cue.instance,
            offset_frames: 0,
            message: CACHE_PLAYER_NOTE_ON,
        })
        .collect()
}

/// 1 小節ぶんの cue を実サーバーへ送る。戻り値は掛かった実時間の内訳。
///
/// **2 パスに分ける。** 先に全 track の `prepare_live_patch`（state load）を済ませ、
/// そのあとで全 track の note on を**まとめて 1 コマンド**で投げる。
/// 1 track ずつ「prepare → note on」と交互に送ると、`prepare` の応答待ち
/// （1 件 10〜13ms・debug サーバーなら 60〜85ms）が note on のあいだに挟まるので、
/// 8 track では最初と最後の note on が 250ms（debug で 650ms）離れて
/// 小節の頭が「ジャラン」と崩れる。実測で見つかった問題（Stage 6）。
///
/// 時間を計って返すのは、**1 小節ごとの state load が小節の中に収まっているか**と
/// **track 間のずれが 1 オーディオブロックに収まっているか**を、あとから数字で
/// 確かめられるようにするため。prepare に失敗した行は note on の対象から外す
/// （音源が載っていないので鳴らしても意味がなく、前の小節の音が出てしまう）。
fn send_measure_cues(
    play_server: &RealtimePlayServerSupervisor,
    cues: &MeasureLiveCues,
    log_lines: &Arc<Mutex<VecDeque<String>>>,
) -> MeasureSendTiming {
    let prepare_started = Instant::now();
    let mut prepared: Vec<LiveCacheCue> = Vec::with_capacity(cues.cues.len());
    for cue in &cues.cues {
        let wav = cue.wav.to_string_lossy().to_string();
        // patch 文字列が `.wav` なので、サーバーは cache-player を選んで instance を差し替える。
        // state に入るのはパスであってファイルの中身ではない（1 ファイル約 1.6MB あるため）。
        if let Err(error) = play_server.prepare_live_patch(cue.instance, Some(&wav)) {
            crate::append_log_line(
                log_lines,
                format!(
                    "live-cache: prepare failed row={} instance={} error=\"{error:#}\"",
                    cue.row, cue.instance
                ),
            );
            continue;
        }
        prepared.push(cue.clone());
    }
    let prepare = prepare_started.elapsed();

    let note_on_started = Instant::now();
    let events = note_on_events(&prepared);
    // 空のバッチはサーバーが `InvalidPayload` で弾く（1..=128 件しか受けない）。
    // 「鳴らすものが無い小節」は正常な状態なので、送らずに黙って抜ける。
    if !events.is_empty() {
        if let Err(error) = play_server.send_live_events(&events) {
            let rows = join_or_dash(prepared.iter().map(|cue| format!("row{}", cue.row)));
            crate::append_log_line(
                log_lines,
                format!("live-cache: note on failed rows={rows} error=\"{error:#}\""),
            );
        }
    }
    MeasureSendTiming {
        prepare,
        note_on: note_on_started.elapsed(),
    }
}

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
    pub(crate) tracks: usize,
    /// `(0 始まりの小節 index, グリッドの行 index)` → いま鳴らせるキャッシュ WAV。
    #[allow(clippy::type_complexity)]
    pub(crate) ready_cache_wav: Arc<dyn Fn(usize, usize) -> Option<PathBuf> + Send + Sync>,
    /// 演奏開始時に送る mixer の gain（演奏開始を決めたときのスナップショット）。
    pub(crate) initial_track_gains: Vec<LiveTrackGain>,
    /// live mix へ最後に送った gain。演奏中の mixer 操作と共有する。
    pub(crate) sent_track_gains: Arc<Mutex<Vec<LiveTrackGain>>>,
}

impl LiveCachePlayLoop {
    /// 停止されるまで小節を進めながら鳴らし続ける。呼んだスレッドを占有する。
    pub(crate) fn run(self, start_measure_index: usize) {
        // サーバーの auto gain は「実際に鳴った音の RMS」から補正値を決めるので、
        // mixer で下げたぶんを打ち消しに来る。DAW の mixer は「ユーザーが決めた音量」
        // なので、live 演奏のあいだは切っておく。
        // grid sequencer は自分の演奏開始時に必ず on へ戻す（`prepare_connection`）ので、
        // ここで off のまま終わっても grid sequencer 側の auto gain は壊れない。
        if let Err(error) = self.play_server.set_live_auto_gain_enabled(false) {
            crate::append_log_line(
                &self.log_lines,
                format!("live-cache: auto gain off failed error=\"{error:#}\""),
            );
        }
        self.apply_initial_track_gains();

        let mut measure_index = start_measure_index;

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
            let measure_start = Instant::now();
            *self.play_position.lock().unwrap() = Some(PlayPosition {
                measure_index: current_measure_index,
                measure_start,
                measure_duration,
            });
            crate::append_log_line(
                &self.log_lines,
                format_playback_measure_resolution_log(
                    measure_index,
                    current_measure_index,
                    effective_count,
                ),
            );

            let cues = measure_live_cues(self.tracks, |row| {
                (self.ready_cache_wav)(current_measure_index, row)
            });
            let timing = send_measure_cues(&self.play_server, &cues, &self.log_lines);
            crate::append_log_line(
                &self.log_lines,
                format_live_cache_measure_log(current_measure_index, &cues, timing),
            );

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
            let next_measure_index = following_measure_index(
                current_measure_index,
                next_effective_count,
                next_ab_repeat_range,
            );
            crate::append_log_line(
                &self.log_lines,
                format_playback_measure_advance_log(
                    current_measure_index,
                    next_measure_index,
                    next_effective_count,
                ),
            );
            measure_index = next_measure_index;
        }

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
            tracks: self.editor.tracks,
            ready_cache_wav: Arc::new(move |measure_index, row| {
                ready_cache_wav_for_measure(workspace_kind, measure_index, row)
            }),
            initial_track_gains: self.desired_live_track_gains(),
            sent_track_gains: Arc::clone(&self.playback.live_track_gains),
        };

        std::thread::spawn(move || play_loop.run(start_measure_index));
    }
}

#[cfg(test)]
mod tests;
