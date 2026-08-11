use std::{collections::HashMap, time::Instant};

use cmrt_arpeggiator::{ArpPattern, BassPattern};
use cmrt_rhythm::DrumPattern;
use cmrt_tui_core::bpm::{BpmInput, BpmMode};

use super::{patch_bag::PatchBag, GridMidiSender, GridPatchStatus, GridState};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PatternEvolution {
    #[default]
    Auto,
    Hold,
}

impl PatternEvolution {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "AUTO",
            Self::Hold => "HOLD",
        }
    }
}

/// サンプルレート未指定時の既定値（config.toml の既定と同じ）。
const DEFAULT_SAMPLE_RATE: f64 = 48_000.0;
/// 出力バッファ1単位のフレーム数の既定値（config.toml の既定と同じ）。
const DEFAULT_BUFFER_FRAMES: usize = 512;

/// 画面を組み立てるときに共有ランタイムから受け取る設定一式。
///
/// 引数が増えても呼び出し側が読めるように、`NotepadScreenParts` と同じく構造体で渡す。
pub struct GridSequencerParts {
    /// `None` ならテストモード（MIDI を送らない）。
    pub midi_sender: Option<GridMidiSender>,
    /// live MIDI の offset をフレーム数で組み立てるためのサンプルレート。
    /// realtime play server が使う config.toml の `sample_rate` と一致させること。
    pub sample_rate: f64,
    /// 出力バッファ1単位のフレーム数（config.toml の `buffer_size`）。
    /// 想定レイテンシの表示にだけ使う。これも server 側の設定と一致させること。
    pub buffer_frames: usize,
    pub track_count: usize,
    /// 前回終了時の chord mode。true なら patch 一覧が揃い次第 on にする。
    pub chord_enabled: bool,
    pub bpm_mode: BpmMode,
    /// 前回終了時に保存した手入力 grid。`None` なら初回入場時にランダム生成する。
    pub restored_session: Option<crate::GridSequencerSession>,
}

impl Default for GridSequencerParts {
    fn default() -> Self {
        Self {
            midi_sender: None,
            sample_rate: DEFAULT_SAMPLE_RATE,
            buffer_frames: DEFAULT_BUFFER_FRAMES,
            track_count: crate::GRID_ROWS,
            chord_enabled: false,
            bpm_mode: BpmMode::Auto,
            restored_session: None,
        }
    }
}

/// grid sequencer 画面が所有する接続・演奏・表示状態。
pub struct GridSequencerScreen {
    /// `None` ならテストモード（MIDI を送らない）。
    pub midi_sender: Option<GridMidiSender>,
    pub state: GridState,
    /// live MIDI の offset をフレーム数で組み立てるためのサンプルレート。
    /// realtime play server が使う config.toml の `sample_rate` と一致させること。
    pub(crate) sample_rate: f64,
    /// 出力バッファ1単位のフレーム数。想定レイテンシの表示にだけ使う。
    pub(crate) buffer_frames: usize,
    /// Non-zero while the server and this screen share an absolute musical epoch.
    pub(crate) timeline_id: u64,
    pub help_open: bool,
    pub(crate) bpm_mode: BpmMode,
    pub(crate) bpm_input: Option<BpmInput>,
    /// cycle ごとに譜面を再抽選するか、人間の編集を保持するか。
    pub(crate) pattern_evolution: PatternEvolution,
    /// mouse down から up まで継続するnote eventの描画・消去操作。
    pub(crate) note_gesture: Option<crate::input::NoteGesture>,
    /// PATCH 欄から開く、行単位の音色選択 overlay。
    pub(crate) patch_selector: Option<crate::patch_selector::PatchSelector>,
    /// note gesture の mouse down 直前に確保した undo 候補。
    pub(crate) pending_undo: Option<crate::undo::UndoSnapshot>,
    /// 直前の論理操作を戻す1段 undo。
    pub(crate) undo: Option<crate::undo::UndoSnapshot>,
    /// 直近のランダム化時点での patch 一覧の状態。ステータス行の表示だけに使う。
    pub patch_status: GridPatchStatus,
    /// 一度でも grid を作ったか。2回目以降の入場で前回の grid を残すために見る。
    pub(crate) grid_ready: bool,
    /// 復元 patch を現在の catalog とまだ照合していない。
    pub(crate) restored_patches_pending: bool,
    /// chord mode を開始／継続できなかった理由。コード進行行に出す。
    pub(crate) chord_error: Option<String>,
    /// chord mode が on か。セッションへ保存する値であり、[`GridState`] ではなく
    /// 画面側が持つ。`t` キーは `state` を作り直すので、そこに置くと切替のたびに落ちる。
    pub(crate) chord_enabled: bool,
    /// セッションから復元した chord mode を、patch 一覧が揃ってから適用するための予約。
    pub(crate) pending_chord: bool,
    /// コード進行データ更新の再起動アナウンスを出し始めた時刻。
    pub(crate) restart_notice: Option<Instant>,
    /// 待機 bank への先読みロードの進み具合。`None` なら先読みしていない。
    pub(crate) cycle_swap: Option<crate::cycle_swap::CycleSwap>,
    /// 音色ロードの完了を待っている。詳細は [`crate::start_wait`]。
    pub(crate) waiting_for_patches: bool,
    /// `Ready` へ戻ってから鳴らし始める時刻。待ちに入った時点では未定。
    pub(crate) resume_at: Option<Instant>,
    /// シングルバッファリングへ落ちているか。詳細は [`crate::single_buffer`]。
    pub(crate) single_buffering: bool,
    /// 自動判定を一度だけ適用するための記録。手動で戻したあと蒸し返さないために持つ。
    pub(crate) overload_applied: bool,
    /// サイクルを鳴らしきってから音色ロードへ入る時刻。`None` なら待っていない。
    pub(crate) cycle_end_at: Option<Instant>,
    /// instance ごとの、直近に適用したアルペジオ音型。wheel の種別送りカーソル。
    /// 譜面そのものは `state` 側に入るので、セッションへは保存しない。
    pub(crate) arp_patterns: HashMap<usize, ArpPattern>,
    /// 直近に適用したアルペジオ音型。NOTE grid のタイトルに出すだけ。
    pub(crate) last_arp: Option<ArpPattern>,
    /// 直近に適用したベースラインの型。wheel の種別送りカーソル。bass 行は1本しか
    /// 無いので instance ごとには持たない。譜面そのものは `state` 側に入る。
    pub(crate) bass_pattern: Option<BassPattern>,
    /// 直近に適用したベースラインの型。NOTE grid のタイトルに出すだけ。
    pub(crate) last_bass: Option<BassPattern>,
    /// instance ごとの、直近に適用した drum のリズム型。wheel の種別送りカーソル。
    /// 譜面そのものは `state` 側に入るので、セッションへは保存しない。
    pub(crate) drum_patterns: HashMap<usize, DrumPattern>,
    /// 直近に適用した drum のリズム型。NOTE grid のタイトルと右 pane に出すだけ。
    pub(crate) last_drum: Option<DrumPattern>,
    /// instance ごとの、PATCH 欄の wheel が辿る patch list。詳細は [`crate::patch_bag`]。
    /// 適用した patch は `state` 側に入るので、セッションへは保存しない。
    pub(crate) patch_bags: HashMap<usize, PatchBag>,
}

impl GridSequencerScreen {
    /// テスト用。既定の設定で作る。
    pub fn new(midi_sender: Option<GridMidiSender>) -> Self {
        Self::new_with(GridSequencerParts {
            midi_sender,
            ..GridSequencerParts::default()
        })
    }

    /// テスト用。track 数だけ指定して作る。
    pub fn with_track_count(midi_sender: Option<GridMidiSender>, track_count: usize) -> Self {
        Self::new_with(GridSequencerParts {
            midi_sender,
            track_count,
            ..GridSequencerParts::default()
        })
    }

    pub fn new_with(parts: GridSequencerParts) -> Self {
        let GridSequencerParts {
            midi_sender,
            sample_rate,
            buffer_frames,
            track_count,
            chord_enabled,
            bpm_mode,
            restored_session,
        } = parts;
        let track_count = cmrt_realtime_play::normalize_live_instance_count(track_count);
        let restored_session = restored_session.filter(|session| !session.instances.is_empty());
        let restored = restored_session.is_some();
        let (state, pattern_evolution) = if let Some(session) = restored_session {
            let mut instances = session.instances;
            let saved_count = instances.len();
            while instances.len() < track_count {
                instances.push(crate::GridInstance::new(instances.len()));
            }
            instances.truncate(track_count);
            // 保存値が track 数に足りないぶんは譜面を抽選して埋める。空のまま足すと
            // HOLD では引き直しが走らず、その行が無音のままになる。patch 一覧はまだ
            // 読み込み中なので、音色は後から `fill_missing_patches` が当てる。
            if let Some(added) = instances.get_mut(saved_count..) {
                crate::randomize_instance_slice(added, &[]);
            }
            let mut state = GridState::with_instance_count(track_count);
            let restored = state.restore_instances(instances);
            debug_assert!(restored);
            (state, session.pattern_evolution)
        } else {
            (
                GridState::with_instance_count(track_count),
                PatternEvolution::Auto,
            )
        };
        Self {
            midi_sender,
            state,
            sample_rate,
            buffer_frames,
            timeline_id: 0,
            help_open: false,
            bpm_mode,
            bpm_input: None,
            pattern_evolution,
            note_gesture: None,
            patch_selector: None,
            pending_undo: None,
            undo: None,
            patch_status: GridPatchStatus::default(),
            grid_ready: restored,
            restored_patches_pending: restored,
            chord_error: None,
            chord_enabled,
            pending_chord: chord_enabled,
            restart_notice: None,
            cycle_swap: None,
            waiting_for_patches: false,
            resume_at: None,
            single_buffering: false,
            overload_applied: false,
            cycle_end_at: None,
            arp_patterns: HashMap::new(),
            last_arp: None,
            bass_pattern: None,
            last_bass: None,
            drum_patterns: HashMap::new(),
            last_drum: None,
            patch_bags: HashMap::new(),
        }
    }

    /// コード進行行に出す chord mode のエラー。
    pub fn chord_error(&self) -> Option<&str> {
        self.chord_error.as_deref()
    }

    pub fn bpm_mode(&self) -> BpmMode {
        self.bpm_mode
    }

    pub fn bpm(&self) -> f64 {
        self.bpm_mode.resolve(crate::BPM)
    }

    /// chord progression またはエラーの1行を描画するか。input layout も同じ判定を使う。
    pub(crate) fn chord_line_visible(&self) -> bool {
        self.chord_error.is_some() || self.state.chord().is_some()
    }

    /// chord mode が on か。セッションへ保存する値。
    pub fn chord_enabled(&self) -> bool {
        self.chord_enabled
    }

    pub fn pattern_evolution(&self) -> PatternEvolution {
        self.pattern_evolution
    }

    /// シングルバッファリングへ落ちているか。ステータス行の表示に使う。
    pub fn single_buffering(&self) -> bool {
        self.single_buffering
    }

    /// 想定レイテンシ（出力バッファに溜める目標時間）。
    ///
    /// サーバーはリングを `buffer_frames * multiplier` フレームまで溜めてから出すので、
    /// 送ってから鳴るまでの遅れはおおよそこの長さになる。
    pub(crate) fn buffer_latency_ms(&self, multiplier: u16) -> f64 {
        if self.sample_rate <= 0.0 {
            return 0.0;
        }
        (self.buffer_frames * usize::from(multiplier)) as f64 / self.sample_rate * 1000.0
    }

    /// 再起動アナウンスを表示中かどうか。
    pub fn restart_notice_open(&self) -> bool {
        self.restart_notice.is_some()
    }

    pub fn track_count(&self) -> usize {
        self.state.instance_count()
    }
}
