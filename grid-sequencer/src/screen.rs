use std::{collections::HashMap, time::Instant};

use cmrt_arpeggiator::{ArpPattern, BassPattern};
use cmrt_rhythm::DrumPattern;
use cmrt_tui_core::bpm::{BpmInput, BpmMode, BpmRange};

use super::{
    cycle_random::CycleRandomOverlay, patch_bag::PatchBag, playback_sync::PlaybackSync,
    CycleRandom, DrawnPhrases, GridMidiSender, GridPatchStatus, GridState,
};

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
    /// 自動BPMを引く範囲。既定は幅を持たない `BPM` 固定。
    pub bpm_range: BpmRange,
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
            bpm_mode: BpmMode::Auto(crate::BPM),
            bpm_range: BpmRange::fixed(crate::BPM),
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
    /// track 単位操作の対象。note のセルカーソルとは別の、軽量な選択状態。
    pub(crate) selected_track: usize,
    /// 直接操作するのは Solo だけ。1つでも true なら、それ以外が派生 mute になる。
    pub(crate) solo_tracks: Vec<bool>,
    /// サーバーが drop 中に止めた musical clock を表示へ反映する同期状態。
    pub(crate) playback_sync: PlaybackSync,
    pub help_open: bool,
    pub(crate) history: crate::history::GridHistory,
    pub(crate) bpm_mode: BpmMode,
    /// 自動BPMを引く範囲。セッションへ保存するのはこちらで、引いた値ではない。
    pub(crate) bpm_range: BpmRange,
    pub(crate) bpm_input: Option<BpmInput>,
    /// `i` キーで開く固定コード進行の1行入力overlay。
    pub(crate) chord_input: Option<crate::chord_input::ChordInputOverlay>,
    /// コード進行1周ごとに何を引き直すか。詳細は [`crate::cycle_random`]。
    pub(crate) cycle_random: CycleRandom,
    /// `a` キーで開く、1周ごとの random 設定 overlay。`None` なら閉じている。
    pub(crate) cycle_random_overlay: Option<CycleRandomOverlay>,
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
    /// 手入力で固定したコード進行。元入力を保存し、復元時にも同じparserへ通す。
    pub(crate) fixed_chord: Option<crate::FixedChordProgression>,
    /// コード進行データ更新の再起動アナウンスを出し始めた時刻。
    pub(crate) restart_notice: Option<Instant>,
    /// patch selector を開けなかった理由の通知。詳細は [`crate::patch_notice`]。
    pub(crate) patch_notice: Option<crate::patch_notice::PatchNotice>,
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
    /// 直近に適用したベースラインの型。wheel の種別送りカーソル。bass 行は1本しか
    /// 無いので instance ごとには持たない。譜面そのものは `state` 側に入る。
    pub(crate) bass_pattern: Option<BassPattern>,
    /// instance ごとの、直近に適用した drum のリズム型。wheel の種別送りカーソル。
    /// 譜面そのものは `state` 側に入るので、セッションへは保存しない。
    pub(crate) drum_patterns: HashMap<usize, DrumPattern>,
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
            bpm_range,
            restored_session,
        } = parts;
        let track_count = cmrt_realtime_play::normalize_live_instance_count(track_count);
        let restored_session = restored_session.filter(|session| !session.instances.is_empty());
        let restored = restored_session.is_some();
        let (state, mut cycle_random, fixed_chord) = if let Some(session) = restored_session {
            let mut instances = session.instances;
            let saved_count = instances.len();
            while instances.len() < track_count {
                instances.push(crate::GridInstance::new(instances.len()));
            }
            instances.truncate(track_count);
            // 保存値が track 数に足りないぶんは譜面を抽選して埋める。空のまま足すと
            // NOTE を OFF にしている間は引き直しが走らず、その行が無音のままになる。
            // patch 一覧はまだ読み込み中なので、音色は後から `fill_missing_patches` が当てる。
            if let Some(added) = instances.get_mut(saved_count..) {
                crate::randomize_instance_slice(added, &[], CycleRandom::ALL, None, None);
            }
            let mut state = GridState::with_instance_count(track_count);
            let restored = state.restore_instances(instances);
            debug_assert!(restored);
            (state, session.cycle_random, session.fixed_chord)
        } else {
            (
                GridState::with_instance_count(track_count),
                CycleRandom::ALL,
                None,
            )
        };
        // fixed_chord が残っているセッションでは固定指定を正本とする。通常の保存経路では
        // CHORD ON 時に fixed_chord を消すが、手編集された不整合JSONも安全側へ正規化する。
        if fixed_chord.is_some() {
            cycle_random.chord = false;
        }
        Self {
            midi_sender,
            state,
            sample_rate,
            buffer_frames,
            timeline_id: 0,
            selected_track: 0,
            solo_tracks: vec![false; track_count],
            playback_sync: PlaybackSync::default(),
            help_open: false,
            history: crate::history::GridHistory::default(),
            bpm_mode,
            bpm_range,
            bpm_input: None,
            chord_input: None,
            cycle_random,
            cycle_random_overlay: None,
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
            fixed_chord,
            restart_notice: None,
            patch_notice: None,
            cycle_swap: None,
            waiting_for_patches: false,
            resume_at: None,
            single_buffering: false,
            overload_applied: false,
            cycle_end_at: None,
            arp_patterns: HashMap::new(),
            bass_pattern: None,
            drum_patterns: HashMap::new(),
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

    /// 自動BPMを引く範囲。セッションへ保存する値。
    pub fn bpm_range(&self) -> BpmRange {
        self.bpm_range
    }

    pub fn bpm(&self) -> f64 {
        self.bpm_mode.bpm()
    }

    /// chord progression またはエラーの1行を描画するか。input layout も同じ判定を使う。
    pub(crate) fn chord_line_visible(&self) -> bool {
        self.chord_error.is_some() || self.state.display_chord().is_some()
    }

    /// chord mode が on か。セッションへ保存する値。
    pub fn chord_enabled(&self) -> bool {
        self.chord_enabled
    }

    pub fn fixed_chord(&self) -> Option<&crate::FixedChordProgression> {
        self.fixed_chord.as_ref()
    }

    /// 抽選で引いた型を、表示用のカーソル（Phrase pane と NOTE grid のタイトル）へ
    /// 取り込む。
    ///
    /// 手動の引き直しで得た型を、現在の表示と操作カーソルへ即時反映する。
    /// 1周ごとの自動抽選はここを通さず、`GridState` が実発音の締切まで表示を待たせる。
    pub(crate) fn absorb_drawn_phrases(&mut self, drawn: DrawnPhrases) {
        self.state.display_drawn_now(drawn);
        for drum in drawn.drums() {
            if let Some(instance) = self
                .state
                .instances()
                .iter()
                .position(|instance| instance.drum == Some(drum.role()))
            {
                self.drum_patterns.insert(instance, drum);
            }
        }
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

    /// patch selector を開けなかった理由を表示中かどうか。
    pub fn patch_notice_open(&self) -> bool {
        self.patch_notice.is_some()
    }

    pub fn track_count(&self) -> usize {
        self.state.instance_count()
    }
}
