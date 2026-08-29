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
    /// 打ちかけの 1 行を chord 行へ移す。overlay 側は既に閉じてある。
    ///
    /// **破棄ではない。** MML のつもりで打った文字列がコード表記だったとき、
    /// その文字列を捨てずに chord 行の同じ小節へ持っていく。編集中だったセルへは
    /// 何も書かない（書くと 2 節のバグ＝無音のセルがそのまま残る）。
    TransferToChordRow {
        line: String,
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

/// chord 行を試聴するときに借りる演奏 track の文脈。
///
/// chord 行自身は音色も voicing も持たない。実際に chord 行から生成される track と
/// 同じ音にするため、chord 行 init と演奏 track の directive を対で受け取る。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChordPreviewContext {
    pub chord_init: String,
    pub track_directive: String,
    /// 演奏 track init の JSON 以外の MML。`o4` など本番で chord MML の前へ付く部分。
    pub mml_prefix: String,
    /// 入力枠へ表示する演奏 track 名（DAW の `T1` など）。
    pub target_label: String,
}

/// 入力欄に書く言語。見た目だけでなく、打鍵プレビューの変換経路も決める。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum MmlOverlaySyntax {
    /// MML と chord 表記を自動判定する従来の入力欄。
    #[default]
    Mml,
    /// chord 行専用。`None` は編集できるが、借りられる演奏 track が無く試聴できない。
    Chord(Option<ChordPreviewContext>),
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
    /// 入力欄に書く言語と、その試聴文脈。
    pub syntax: MmlOverlaySyntax,
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
    /// 打ちかけの 1 行を chord 行へ移せるか。**chord 行を持つ画面（DAW）だけ `true`。**
    ///
    /// notepad / keyboard / grid から開いた overlay には移送先が無いので、
    /// chord のヒントも確認ダイアログも一切出さない（出しても行き先が無い）。
    pub chord_row_transfer: bool,
    /// 設定不足でカタログから外れたプラグインの案内（`SkippedCatalogPlugin::notice_line`）。
    ///
    /// 「音色一覧に出てこない」は一覧を見ているだけでは絶対に気づけない
    /// （**出ていないものは見えない**）ので、音色選択を開いている間だけ枠の下へ出す。
    /// 空なら 1 行も増えない。
    pub catalog_notes: Vec<String>,
}
