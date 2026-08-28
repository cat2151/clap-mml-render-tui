//! オーバーレイと呼び出し側の契約。
//!
//! 「開くときに何を渡すか」（[`MmlOverlayContext`]）と
//! 「オーバーレイが何をしてほしいか」（[`MmlOverlayAction`]）だけを置く。
//! 状態を持たない定義だけなので、判定を持つ [`super`] から分けてある。

use std::{collections::BTreeMap, time::Duration};

use cmrt_patches::PatchRoleIndex;

use cmrt_tui_core::patch_load::PatchLoadMeasurement;

use crate::line_play::LineProgram;
use crate::PatchCatalogEntry;

/// 生 MIDI の note on と、送信成功後に保つべき音長。
#[derive(Clone, Debug, PartialEq)]
pub struct NoteRequest {
    pub messages: Vec<[u8; 3]>,
    pub duration: Duration,
}

/// 行を鳴らす前に音源の音色を差し替えるか。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchChange {
    /// いまの音色のまま鳴らす。
    Keep,
    /// 鳴らす前にこの音色へ差し替える（`None` は realtime server の既定音色へ戻す）。
    Switch(Option<String>),
}

/// オーバーレイが呼び出し側へ求める処理。
///
/// 音を出す変種（[`Self::Send`] / [`Self::SetPatch`] / [`Self::PlayLine`]）は、
/// どれも「鳴っているものを止めてから鳴らす」の意味になる。止める指示は載せない。
#[derive(Clone, Debug, PartialEq)]
pub enum MmlOverlayAction {
    Continue,
    /// MML patch selector で追加した正規表現プリセットを host app に保存させる。
    SavePatchFilterPresets {
        presets: Vec<(String, String)>,
        /// 絞り込み更新で新しい先頭候補へ移った場合は、保存と同時に試聴する。
        preview: Option<(String, Option<NoteRequest>)>,
    },
    /// 鳴っているものを止めてから、この note on を送る。
    Send(NoteRequest),
    /// 鳴っているものを止め、音源の音色を差し替えてから、この note on を送る。
    /// `patch` が `None` なら realtime server の既定音色へ戻す。
    SetPatch {
        patch: Option<String>,
        notes: Option<NoteRequest>,
    },
    /// 鳴っているものを止め、あらためてこの行を頭から積む。
    /// `program` が空（[`LineProgram::is_silent`]）なら止めるだけ。
    PlayLine {
        patch: PatchChange,
        program: LineProgram,
    },
    /// 1 行モードで入力を確定した。ホストはこの 1 行を書き戻す。
    ///
    /// `close` が `false` なら overlay は開いたまま。ホストは次の対象の内容で
    /// [`MmlOverlay::open`] し直す（DAW なら次の小節）。`close` が `true` なら
    /// overlay 側で既に閉じてあるので、ホストは [`Self::Close`] と同じ後始末をする。
    Commit {
        line: String,
        close: bool,
    },
    /// オーバーレイを閉じる。鳴っているものを止めるのも含む。
    Close,
}

/// 入力欄を何行で開くか。開くときに呼び出し側が決める。
///
/// [`Self::MultiLine`] が従来の挙動（1 行 1 フレーズを書き並べて聴き比べる）。
/// [`Self::SingleLine`] は「1 か所へ書き戻すための入力欄」で、`Enter` が改行では
/// なく確定になる（DAW の小節セル用）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MmlOverlayInputMode {
    #[default]
    MultiLine,
    SingleLine,
}

/// MML overlay が受け取る、plugin 非依存の音色一覧スナップショット。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PatchCatalogSnapshot {
    /// バックグラウンド収集中。Ctrl+T は完了後の open 予約になる。
    #[default]
    Loading,
    /// 収集済みのselector行。空なら選べる音色がない。
    Ready(Vec<PatchCatalogEntry>),
    /// 収集に失敗した理由。Ctrl+T 時に overlay 内へ表示する。
    Error(String),
}

/// オーバーレイを開くときに呼び出し側から渡すスナップショット。
#[derive(Default)]
pub struct MmlOverlayContext {
    /// 入力欄を何行で開くか。既定は従来どおり複数行。
    pub input_mode: MmlOverlayInputMode,
    /// 開いた直後から入力欄に入れておく文字列。
    ///
    /// 複数行モードでは使わない（従来どおり常に空で開く）。1 行モードでは
    /// 改行より後ろを捨てて先頭 1 行だけを入れる。
    pub initial_text: String,
    pub patch_catalog: PatchCatalogSnapshot,
    /// MML selectorとGrid Sequencerが共有する、同じcatalog世代のRole索引。
    pub patch_role_index: PatchRoleIndex,
    /// catalog構築時に計測したpatch別のload結果。
    pub load_measurements: BTreeMap<String, PatchLoadMeasurement>,
    /// notepad 画面と共有しているフレーズ履歴。
    pub history: Vec<String>,
    pub favorites: Vec<String>,
    /// `(Grid Sequencer 上の役割 group, 正規表現)` のユーザー追加プリセット。
    pub patch_filter_presets: Vec<(String, String)>,
    /// 設定不足でカタログから外れたプラグインの案内（`SkippedCatalogPlugin::notice_line`）。
    ///
    /// 「音色一覧に出てこない」は一覧を見ているだけでは絶対に気づけない
    /// （**出ていないものは見えない**）ので、音色選択を開いている間だけ枠の下へ出す。
    /// 空なら 1 行も増えない。
    pub catalog_notes: Vec<String>,
}
