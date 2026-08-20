//! 画面が共有ランタイムから受け取る情報一式と、その読み取り。
//!
//! 画面側は config も patch 一覧も自分では持たない。app 側の glue
//! （`tui::grid_sequencer_glue`）が毎フレーム組み立てて渡す。

use cmrt_chord::ChordProgressionCatalog;
use cmrt_tui_core::patch_plugins::PatchPlugins;

use crate::GridVoicingLookup;

/// patch 一覧のバックグラウンド読み込み状態のスナップショット。
/// 共有ランタイム側の `PatchLoadState` から glue が変換して渡す。
pub enum GridPatchLoad<'a> {
    Loading,
    Ready(&'a [(String, String)]),
    Err(&'a str),
}

/// grid sequencer 画面が共有ランタイムから受け取る情報一式。
pub struct GridSequencerContext<'a> {
    pub patch_dirs_configured: bool,
    pub patch_load: GridPatchLoad<'a>,
    /// chord mode が進行を抽選するカタログ。空なら chord mode は開始できない。
    pub chord_catalog: &'a ChordProgressionCatalog,
    /// 和音用 patch の当たり判定に使う mono/poly 判定。
    pub voicing: &'a dyn GridVoicingLookup,
    /// patch 文字列から「その音色を鳴らすプラグイン」と、そのプラグイン向けの
    /// 用途別カテゴリ／キーワードを引く表。
    ///
    /// カタログに複数プラグインの音色が並ぶと絞り込みは 1 組では足りないので、
    /// この画面は config の 7 項目を直接は見ず、必ずここを通す。
    pub patch_plugins: &'a PatchPlugins,
    /// コード進行カタログが更新されたか（再起動アナウンスの合図。一度だけ true）。
    pub chord_source_updated: bool,
}

impl GridSequencerContext<'_> {
    /// ランダム選択に使える patch 一覧。読み込み中・エラー時は空を返す。
    pub(crate) fn patches(&self) -> &[(String, String)] {
        match &self.patch_load {
            GridPatchLoad::Ready(pairs) => pairs,
            GridPatchLoad::Loading | GridPatchLoad::Err(_) => &[],
        }
    }

    pub(crate) fn patch_status(&self) -> GridPatchStatus {
        if !self.patch_dirs_configured {
            return GridPatchStatus::NotConfigured;
        }
        match &self.patch_load {
            GridPatchLoad::Ready(pairs) => GridPatchStatus::Ready(pairs.len()),
            GridPatchLoad::Loading => GridPatchStatus::Loading,
            GridPatchLoad::Err(error) => GridPatchStatus::Err((*error).to_string()),
        }
    }

    pub(crate) fn patches_are_loading(&self) -> bool {
        matches!(self.patch_load, GridPatchLoad::Loading)
    }

    pub(crate) fn patches_are_ready(&self) -> bool {
        matches!(self.patch_load, GridPatchLoad::Ready(_))
    }
}

/// ステータス行に出す patch 一覧の状態（直近のランダム化時点のスナップショット）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum GridPatchStatus {
    #[default]
    Loading,
    Ready(usize),
    NotConfigured,
    Err(String),
}

impl GridPatchStatus {
    pub fn label(&self) -> String {
        match self {
            Self::Loading => "patches loading".to_string(),
            Self::Ready(count) => format!("{count} patches"),
            Self::NotConfigured => "patches_dirs 未設定".to_string(),
            Self::Err(error) => format!("patches error: {error}"),
        }
    }
}
